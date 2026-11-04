use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::service::{self, AppState, ImportResult, Snapshot};

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
