use std::io;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use tauri::{App, Emitter, Manager, Runtime};

use crate::core::model::{Binding, Data, Link, Settings};
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

    fn temp_state(data: Data) -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        store::save(&path, &data).unwrap();
        (dir, AppState::new(data, path, None))
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
}
