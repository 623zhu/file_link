use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::service::{self, AppState, Snapshot};

#[tauri::command]
pub fn get_snapshot(app: AppHandle, state: State<AppState>) -> Result<Snapshot, AppError> {
    service::emit_recovery_if_needed(&state, &app)?;
    service::snapshot(&state)
}
