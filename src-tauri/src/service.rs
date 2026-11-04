use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use chrono::Local;
use serde::Serialize;
use tauri::{App, Emitter, Manager, Runtime};
use uuid::Uuid;

use crate::core::model::{Binding, Data, Ignore, Link, Settings, Source};
use crate::core::path;
use crate::core::reconcile::{self, ReconcileSummary};
use crate::core::rule;
use crate::core::scanner;
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

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoveLinksResult {
    pub removed: usize,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewBindingResult {
    pub count: usize,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddBindingResult {
    pub binding: Binding,
    pub added: usize,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoveBindingResult {
    pub removed_links: usize,
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

pub fn link_path(state: &AppState, id: &str) -> Result<String, AppError> {
    let data = lock(&state.data)?;
    data.links
        .iter()
        .find(|link| link.id == id)
        .map(|link| link.path.clone())
        .ok_or(AppError::NotFound)
}

pub fn mark_missing(data: &mut Data, id: &str) -> Result<(), AppError> {
    let link = data
        .links
        .iter_mut()
        .find(|link| link.id == id)
        .ok_or(AppError::NotFound)?;
    link.missing = true;
    Ok(())
}

pub fn remove_links(data: &mut Data, ids: &[String]) -> RemoveLinksResult {
    let mut ignores = Vec::new();
    let before = data.links.len();
    data.links.retain(|link| {
        if !ids.iter().any(|id| id == &link.id) {
            return true;
        }
        for source in &link.sources {
            if let Source::Binding { binding_id } = source {
                ignores.push(Ignore {
                    binding_id: binding_id.clone(),
                    key: link.key.clone(),
                });
            }
        }
        false
    });
    for ignore in ignores {
        if !data
            .ignored
            .iter()
            .any(|existing| existing.binding_id == ignore.binding_id && existing.key == ignore.key)
        {
            data.ignored.push(ignore);
        }
    }
    RemoveLinksResult {
        removed: before - data.links.len(),
    }
}

pub fn preview_binding(
    state: &AppState,
    raw_path: &str,
    recursive: bool,
) -> Result<PreviewBindingResult, AppError> {
    let (display, key) = normalize_dir(raw_path)?;
    let settings = lock(&state.data)?.settings.clone();
    let binding = new_binding(display.clone(), key, recursive, false);
    let files = scan_files(&binding, &settings)?;
    Ok(PreviewBindingResult { count: files.len() })
}

pub fn add_binding(
    state: &AppState,
    raw_path: &str,
    recursive: bool,
    auto_watch: bool,
) -> Result<AddBindingResult, AppError> {
    let (display, key) = normalize_dir(raw_path)?;
    let settings = {
        let data = lock(&state.data)?;
        if data.bindings.iter().any(|binding| binding.key == key) {
            return Err(AppError::BindingExists);
        }
        data.settings.clone()
    };
    let binding = new_binding(display, key, recursive, auto_watch);
    let files = scan_files(&binding, &settings)?;
    write(state, |data| {
        if data
            .bindings
            .iter()
            .any(|existing| existing.key == binding.key)
        {
            return Err(AppError::BindingExists);
        }
        let added_binding = binding.clone();
        data.bindings.push(binding);
        let summary = reconcile::reconcile(data, &added_binding.id, &files, file_exists);
        Ok(AddBindingResult {
            binding: added_binding,
            added: summary.added,
        })
    })
}

pub fn update_binding(
    state: &AppState,
    id: &str,
    recursive: Option<bool>,
    auto_watch: Option<bool>,
) -> Result<Binding, AppError> {
    let (mut binding, settings) = {
        let data = lock(&state.data)?;
        (find_binding(&data, id)?.clone(), data.settings.clone())
    };
    if let Some(recursive) = recursive {
        binding.recursive = recursive;
    }
    if let Some(auto_watch) = auto_watch {
        binding.auto_watch = auto_watch;
    }
    let files = if recursive.is_some() {
        Some(scan_files(&binding, &settings))
    } else {
        None
    };

    write(state, |data| {
        {
            let current = data
                .bindings
                .iter_mut()
                .find(|binding| binding.id == id)
                .ok_or(AppError::NotFound)?;
            current.recursive = binding.recursive;
            current.auto_watch = binding.auto_watch;
            match &files {
                Some(Ok(_)) => current.unavailable = false,
                Some(Err(_)) => current.unavailable = true,
                None => {}
            }
        }
        if let Some(Ok(files)) = &files {
            reconcile::reconcile(data, id, files, file_exists);
        }
        find_binding(data, id).cloned()
    })
}

pub fn remove_binding(state: &AppState, id: &str) -> Result<RemoveBindingResult, AppError> {
    write(state, |data| {
        if !data.bindings.iter().any(|binding| binding.id == id) {
            return Err(AppError::NotFound);
        }
        data.bindings.retain(|binding| binding.id != id);
        Ok(RemoveBindingResult {
            removed_links: reconcile::unbind(data, id),
        })
    })
}

pub fn scan_binding(state: &AppState, id: &str) -> Result<ReconcileSummary, AppError> {
    let (binding, settings) = {
        let data = lock(&state.data)?;
        (find_binding(&data, id)?.clone(), data.settings.clone())
    };
    match scan_files(&binding, &settings) {
        Ok(files) => write(state, |data| {
            {
                let current = data
                    .bindings
                    .iter_mut()
                    .find(|binding| binding.id == id)
                    .ok_or(AppError::NotFound)?;
                current.unavailable = false;
            }
            Ok(reconcile::reconcile(data, id, &files, file_exists))
        }),
        Err(err) => {
            let _ = write(state, |data| {
                if let Some(current) = data.bindings.iter_mut().find(|binding| binding.id == id) {
                    current.unavailable = true;
                }
                Ok(())
            });
            Err(err)
        }
    }
}

pub fn scan_all(state: &AppState) -> Result<ReconcileSummary, AppError> {
    let (settings, bindings) = {
        let data = lock(&state.data)?;
        (data.settings.clone(), data.bindings.clone())
    };
    let jobs: Vec<_> = bindings
        .iter()
        .map(|binding| {
            (
                binding.id.clone(),
                scan_files(binding, &settings).map_err(|_| ()),
            )
        })
        .collect();

    write(state, |data| {
        let mut summary = ReconcileSummary::default();
        for (id, scanned) in &jobs {
            match scanned {
                Ok(files) => {
                    if let Some(binding) =
                        data.bindings.iter_mut().find(|binding| binding.id == *id)
                    {
                        binding.unavailable = false;
                    }
                    let part = reconcile::reconcile(data, id, files, file_exists);
                    summary.added += part.added;
                    summary.restored += part.restored;
                    summary.missing += part.missing;
                }
                Err(()) => {
                    if let Some(binding) =
                        data.bindings.iter_mut().find(|binding| binding.id == *id)
                    {
                        binding.unavailable = true;
                    }
                }
            }
        }
        reconcile::refresh_missing(data, file_exists);
        Ok(summary)
    })
}

fn find_binding<'a>(data: &'a Data, id: &str) -> Result<&'a Binding, AppError> {
    data.bindings
        .iter()
        .find(|binding| binding.id == id)
        .ok_or(AppError::NotFound)
}

fn normalize_dir(raw: &str) -> Result<(String, String), AppError> {
    let (display, key) = path::normalize(raw).map_err(|_| AppError::BindingUnavailable)?;
    if !Path::new(&display).is_dir() {
        return Err(AppError::BindingUnavailable);
    }
    Ok((display, key))
}

fn new_binding(display: String, key: String, recursive: bool, auto_watch: bool) -> Binding {
    Binding {
        id: Uuid::new_v4().to_string(),
        path: display,
        key,
        recursive,
        auto_watch,
        rule_id: None,
        created_at: Local::now(),
        unavailable: false,
    }
}

fn scan_files(binding: &Binding, settings: &Settings) -> Result<Vec<(String, String)>, AppError> {
    let rule = rule::effective_rule(binding, settings);
    scanner::scan(&binding.path, binding.recursive, &rule).map_err(|_| AppError::BindingUnavailable)
}

fn file_exists(path: &str) -> bool {
    Path::new(path).is_file()
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

    fn sample_link(id: &str, key: &str, sources: Vec<Source>) -> Link {
        Link {
            id: id.to_string(),
            path: format!("D:\\files\\{id}.html"),
            key: key.to_string(),
            name: format!("{id}.html"),
            sources,
            added_at: Local::now(),
            missing: false,
        }
    }

    #[test]
    fn remove_links_does_not_ignore_manual_links() {
        let mut data = Data::default();
        data.links.push(sample_link(
            "manual",
            "d:\\files\\manual.html",
            vec![Source::Manual],
        ));

        let result = remove_links(&mut data, &["manual".to_string()]);

        assert_eq!(result, RemoveLinksResult { removed: 1 });
        assert!(data.links.is_empty());
        assert!(data.ignored.is_empty());
    }

    #[test]
    fn remove_links_writes_ignore_for_each_binding_source() {
        let mut data = Data::default();
        data.links.push(sample_link(
            "bound",
            "d:\\files\\bound.html",
            vec![
                Source::Manual,
                Source::Binding {
                    binding_id: "b1".to_string(),
                },
                Source::Binding {
                    binding_id: "b2".to_string(),
                },
            ],
        ));

        let result = remove_links(&mut data, &["bound".to_string()]);

        assert_eq!(result, RemoveLinksResult { removed: 1 });
        assert!(data.links.is_empty());
        assert_eq!(
            data.ignored,
            [
                Ignore {
                    binding_id: "b1".to_string(),
                    key: "d:\\files\\bound.html".to_string(),
                },
                Ignore {
                    binding_id: "b2".to_string(),
                    key: "d:\\files\\bound.html".to_string(),
                },
            ]
        );
    }

    #[test]
    fn add_binding_rejects_duplicate_directory() {
        let dir = tempfile::tempdir().unwrap();
        write_html(dir.path(), "a.html");
        let (_data_dir, state) = temp_state(Data::default());
        let path = dir.path().to_string_lossy().into_owned();

        add_binding(&state, &path, false, false).unwrap();
        let err = add_binding(&state, &path, true, true).unwrap_err();

        assert!(matches!(err, AppError::BindingExists));
        assert_eq!(snapshot(&state).unwrap().bindings.len(), 1);
    }

    #[test]
    fn update_binding_recursive_changes_reconcile_results() {
        let dir = tempfile::tempdir().unwrap();
        write_html(dir.path(), "root.html");
        let nested = dir.path().join("sub");
        fs::create_dir(&nested).unwrap();
        write_html(&nested, "nested.html");
        let (_data_dir, state) = temp_state(Data::default());
        let path = dir.path().to_string_lossy().into_owned();

        let added = add_binding(&state, &path, false, false).unwrap();
        assert_eq!(added.added, 1);
        assert_eq!(snapshot(&state).unwrap().links.len(), 1);

        let updated = update_binding(&state, &added.binding.id, Some(true), None).unwrap();
        assert!(updated.recursive);
        let names: Vec<_> = snapshot(&state)
            .unwrap()
            .links
            .iter()
            .map(|link| link.name.clone())
            .collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"root.html".to_string()));
        assert!(names.contains(&"nested.html".to_string()));
    }

    #[test]
    fn scan_all_continues_when_one_binding_is_unavailable() {
        let ok_dir = tempfile::tempdir().unwrap();
        write_html(ok_dir.path(), "ok.html");
        let gone_dir = tempfile::tempdir().unwrap();
        write_html(gone_dir.path(), "gone.html");
        let (_data_dir, state) = temp_state(Data::default());

        let ok = add_binding(&state, &ok_dir.path().to_string_lossy(), false, false).unwrap();
        let gone = add_binding(&state, &gone_dir.path().to_string_lossy(), false, false).unwrap();
        drop(gone_dir);

        let summary = scan_all(&state).unwrap();
        let snap = snapshot(&state).unwrap();
        let gone_binding = snap
            .bindings
            .iter()
            .find(|binding| binding.id == gone.binding.id)
            .unwrap();
        let ok_binding = snap
            .bindings
            .iter()
            .find(|binding| binding.id == ok.binding.id)
            .unwrap();

        assert!(gone_binding.unavailable);
        assert!(!ok_binding.unavailable);
        assert_eq!(summary.added, 0);
        assert!(
            snap.links
                .iter()
                .any(|link| link.name == "ok.html" && !link.missing)
        );
    }
}
