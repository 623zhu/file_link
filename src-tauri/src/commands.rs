use std::path::Path;

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::AppError;
use crate::service::{self, AppState, ImportResult, RemoveLinksResult, Snapshot};

#[tauri::command]
pub fn get_snapshot(app: AppHandle, state: State<AppState>) -> Result<Snapshot, AppError> {
    service::emit_recovery_if_needed(&state, &app)?;
    service::snapshot(&state)
}

#[tauri::command]
pub fn import_files(
    app: AppHandle,
    state: State<AppState>,
    paths: Vec<String>,
) -> Result<ImportResult, AppError> {
    service::write_and_emit(&state, &app, |data| Ok(service::import_files(data, &paths)))
}

#[tauri::command]
pub fn open_link(app: AppHandle, state: State<AppState>, id: String) -> Result<(), AppError> {
    let path = service::link_path(&state, &id)?;
    if Path::new(&path).is_file() {
        return app
            .opener()
            .open_path(path, None::<&str>)
            .map_err(open_failed);
    }
    service::write_and_emit(&state, &app, |data| service::mark_missing(data, &id))?;
    Err(AppError::FileMissing)
}

#[tauri::command]
pub fn reveal_link(app: AppHandle, state: State<AppState>, id: String) -> Result<(), AppError> {
    let path = service::link_path(&state, &id)?;
    let path = Path::new(&path);
    if path.is_file() {
        return app.opener().reveal_item_in_dir(path).map_err(open_failed);
    }
    if let Some(parent) = path.parent().filter(|parent| parent.is_dir()) {
        return app
            .opener()
            .open_path(parent.to_string_lossy().into_owned(), None::<&str>)
            .map_err(open_failed);
    }
    Err(AppError::FileMissing)
}

#[tauri::command]
pub fn remove_links(
    app: AppHandle,
    state: State<AppState>,
    ids: Vec<String>,
) -> Result<RemoveLinksResult, AppError> {
    service::write_and_emit(&state, &app, |data| Ok(service::remove_links(data, &ids)))
}

fn open_failed(err: impl ToString) -> AppError {
    AppError::OpenFailed(err.to_string())
}
