use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use chrono::Local;
use serde::Serialize;
use tauri::{App, Emitter, Manager, Runtime};
use uuid::Uuid;

use crate::core::model::{Binding, Data, Link, Settings, Source};
use crate::core::path;
use crate::core::store;
use crate::error::AppError;

pub struct AppState {
    data: Mutex<Data>,
    data_path: PathBuf,
    recovery_path: Mutex<Option<PathBuf>>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub settings: Settings,
    pub bindings: Vec<Binding>,
    pub links: Vec<Link>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub added: usize,
    pub skipped: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DataRecoveredPayload {
    backup_path: String,
}

impl AppState {
    pub fn new(data: Data, data_path: PathBuf, recovery_path: Option<PathBuf>) -> Self {
        Self {
            data: Mutex::new(data),
            data_path,
            recovery_path: Mutex::new(recovery_path),
        }
    }
}

pub fn bootstrap<R: Runtime>(app: &App<R>) -> Result<(), AppError> {
    let data_path = app.path().app_data_dir().map_err(to_io)?.join("data.json");
    let loaded = store::load(&data_path)?;
    store::save(&data_path, &loaded.data)?;
    app.manage(AppState::new(loaded.data, data_path, loaded.backup_path));
    Ok(())
}

pub fn snapshot(state: &AppState) -> Result<Snapshot, AppError> {
    let data = lock(&state.data)?;
    Ok(Snapshot {
        settings: data.settings.clone(),
        bindings: data.bindings.clone(),
        links: data.links.clone(),
    })
}

/// 加锁 → 修改 → 原子保存 → 解锁。
pub fn write<F, T>(state: &AppState, f: F) -> Result<T, AppError>
where
    F: FnOnce(&mut Data) -> Result<T, AppError>,
{
    let mut data = lock(&state.data)?;
    let mut next = data.clone();
    let result = f(&mut next)?;
    store::save(&state.data_path, &next)?;
    *data = next;
    Ok(result)
}

/// 写操作封装：保存成功后再发送 `state-changed`。
pub fn write_and_emit<R, E, F, T>(state: &AppState, emitter: &E, f: F) -> Result<T, AppError>
where
    R: Runtime,
    E: Emitter<R>,
    F: FnOnce(&mut Data) -> Result<T, AppError>,
{
    let result = write(state, f)?;
    emitter.emit("state-changed", ()).map_err(to_io)?;
    Ok(result)
}

pub fn import_files(data: &mut Data, paths: &[String]) -> ImportResult {
    let mut added = 0;
    let mut skipped = 0;
    for path in paths {
        match import_one(data, path) {
            ImportOutcome::Added => added += 1,
            ImportOutcome::Skipped => skipped += 1,
        }
    }
    ImportResult { added, skipped }
}

enum ImportOutcome {
    Added,
    Skipped,
}

fn import_one(data: &mut Data, raw: &str) -> ImportOutcome {
    let (display, key) = match path::normalize(raw) {
        Ok(normalized) => normalized,
        Err(err) => {
            log::warn!("skipping invalid import path {raw}: {err}");
            return ImportOutcome::Skipped;
        }
    };

    if !Path::new(&display).is_file() {
        log::warn!("skipping non-file import path {display}");
        return ImportOutcome::Skipped;
    }

    if let Some(existing) = data.links.iter_mut().find(|link| link.key == key) {
        existing.add_source(Source::Manual);
        return ImportOutcome::Skipped;
    }

    let name = Path::new(&display)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| display.clone());

    data.links.push(Link {
        id: Uuid::new_v4().to_string(),
        path: display,
        key,
        name,
        sources: vec![Source::Manual],
        added_at: Local::now(),
        missing: false,
    });
    ImportOutcome::Added
}

pub fn emit_recovery_if_needed<R: Runtime, E: Emitter<R>>(
    state: &AppState,
    emitter: &E,
) -> Result<(), AppError> {
    let mut recovery = lock(&state.recovery_path)?;
    let Some(path) = recovery.take() else {
        return Ok(());
    };
    drop(recovery);
    emitter
        .emit(
            "data-recovered",
            DataRecoveredPayload {
                backup_path: path.to_string_lossy().into_owned(),
            },
        )
        .map_err(to_io)?;
    Ok(())
}

fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, AppError> {
    mutex.lock().map_err(|_| poisoned())
}

fn poisoned() -> AppError {
    io::Error::other("application state lock poisoned").into()
}

fn to_io(err: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> AppError {
    io::Error::other(err).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::Data;
    use std::fs;

    fn temp_state(data: Data) -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        store::save(&path, &data).unwrap();
        (dir, AppState::new(data, path, None))
    }

    fn write_html(dir: &Path, name: &str) -> String {
        let file = dir.join(name);
        fs::write(&file, "<html></html>").unwrap();
        file.to_string_lossy().into_owned()
    }

    #[test]
    fn write_persists_changes_to_disk() {
        let (_dir, state) = temp_state(Data::default());

        write(&state, |data| {
            data.settings.extensions = vec![".md".to_string()];
            Ok(())
        })
        .unwrap();

        let loaded = store::load(&state.data_path).unwrap();
        assert_eq!(loaded.data.settings.extensions, [".md"]);
        assert_eq!(
            snapshot(&state).unwrap().settings.extensions,
            loaded.data.settings.extensions
        );
    }

    #[test]
    fn write_keeps_old_data_when_mutator_fails() {
        let original = Data::default();
        let (_dir, state) = temp_state(original.clone());

        let err = write(&state, |data| -> Result<(), AppError> {
            data.settings.extensions = vec![".md".to_string()];
            Err(AppError::NotFound)
        })
        .unwrap_err();

        assert!(matches!(err, AppError::NotFound));
        assert_eq!(store::load(&state.data_path).unwrap().data, original);
        assert_eq!(snapshot(&state).unwrap().settings, original.settings);
    }

    #[test]
    fn import_files_skips_duplicate_paths() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_html(dir.path(), "a.html");
        let mut data = Data::default();

        let first = import_files(&mut data, std::slice::from_ref(&path));
        let second = import_files(&mut data, std::slice::from_ref(&path));

        assert_eq!(
            first,
            ImportResult {
                added: 1,
                skipped: 0
            }
        );
        assert_eq!(
            second,
            ImportResult {
                added: 0,
                skipped: 1
            }
        );
        assert_eq!(data.links.len(), 1);
        assert_eq!(data.links[0].sources, [Source::Manual]);
        assert_eq!(data.links[0].name, "a.html");
    }

    #[test]
    fn import_files_treats_case_variants_as_the_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_html(dir.path(), "Note.HTML");
        let mut data = Data::default();

        let first = import_files(&mut data, std::slice::from_ref(&path));
        let second = import_files(&mut data, &[path.to_uppercase()]);

        assert_eq!(first.added, 1);
        assert_eq!(
            second,
            ImportResult {
                added: 0,
                skipped: 1
            }
        );
        assert_eq!(data.links.len(), 1);
    }

    #[test]
    fn import_files_skips_invalid_paths_without_creating_links() {
        let mut data = Data::default();
        let result = import_files(&mut data, &["Z:\\missing\\nope.html".to_string()]);

        assert_eq!(
            result,
            ImportResult {
                added: 0,
                skipped: 1
            }
        );
        assert!(data.links.is_empty());
    }

    #[test]
    fn import_files_adds_manual_source_to_existing_binding_link() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_html(dir.path(), "shared.html");
        let (display, key) = path::normalize(&path).unwrap();
        let mut data = Data::default();
        data.links.push(Link {
            id: "existing".to_string(),
            path: display,
            key,
            name: "shared.html".to_string(),
            sources: vec![Source::Binding {
                binding_id: "b1".to_string(),
            }],
            added_at: Local::now(),
            missing: false,
        });

        let result = import_files(&mut data, std::slice::from_ref(&path));

        assert_eq!(
            result,
            ImportResult {
                added: 0,
                skipped: 1
            }
        );
        assert_eq!(data.links.len(), 1);
        assert!(data.links[0].has_source(&Source::Manual));
        assert!(data.links[0].has_source(&Source::Binding {
            binding_id: "b1".to_string(),
        }));
    }

    #[test]
    fn import_files_persists_new_links() {
        let files_dir = tempfile::tempdir().unwrap();
        let a = write_html(files_dir.path(), "a.html");
        let b = write_html(files_dir.path(), "b.html");
        let (_data_dir, state) = temp_state(Data::default());

        let result = write(&state, |data| Ok(import_files(data, &[a, b]))).unwrap();

        assert_eq!(
            result,
            ImportResult {
                added: 2,
                skipped: 0
            }
        );
        let loaded = store::load(&state.data_path).unwrap();
        assert_eq!(loaded.data.links.len(), 2);
        assert!(
            loaded
                .data
                .links
                .iter()
                .all(|link| link.sources == [Source::Manual])
        );
    }
}
