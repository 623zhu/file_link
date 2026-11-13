use std::io;
use std::path::Path;

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::core::model::{Binding, Settings};
use crate::core::reconcile::ReconcileSummary;
use crate::error::AppError;
use crate::service::{
    self, AddBindingResult, AppState, ImportResult, PreviewBindingResult, RemoveBindingResult,
    RemoveLinksResult, Snapshot,
};

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
    if service::link_has_unavailable_binding(&state, &id)? {
        return Err(AppError::BindingUnavailable);
    }
    if let Some(path) = service::relocate_link(&state, &id)? {
        emit_state_changed(&app)?;
        return app
            .opener()
            .open_path(path, None::<&str>)
            .map_err(open_failed);
    }
    service::write_and_emit(&state, &app, |data| service::remove_gone_link(data, &id))?;
    Err(AppError::FileMissing)
}

#[tauri::command]
pub fn reveal_link(app: AppHandle, state: State<AppState>, id: String) -> Result<(), AppError> {
    let path = service::link_path(&state, &id)?;
    let path = Path::new(&path);
    if path.is_file() {
        return app.opener().reveal_item_in_dir(path).map_err(open_failed);
    }
    let parent = path
        .parent()
        .filter(|parent| parent.is_dir())
        .map(|parent| parent.to_string_lossy().into_owned());
    if !service::link_has_unavailable_binding(&state, &id)? {
        if let Some(found) = service::relocate_link(&state, &id)? {
            emit_state_changed(&app)?;
            return app
                .opener()
                .reveal_item_in_dir(Path::new(&found))
                .map_err(open_failed);
        }
        service::write_and_emit(&state, &app, |data| service::remove_gone_link(data, &id))?;
    }
    if let Some(parent) = parent {
        return app
            .opener()
            .open_path(parent, None::<&str>)
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

#[tauri::command]
pub fn set_starred(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    starred: bool,
) -> Result<(), AppError> {
    service::write_and_emit(&state, &app, |data| {
        service::set_starred(data, &id, starred)
    })
}

#[tauri::command]
pub fn preview_binding(
    state: State<AppState>,
    path: String,
    recursive: bool,
    extensions: Vec<String>,
) -> Result<PreviewBindingResult, AppError> {
    service::preview_binding(&state, &path, recursive, &extensions)
}

#[tauri::command(rename_all = "camelCase")]
pub fn add_binding(
    app: AppHandle,
    state: State<AppState>,
    path: String,
    recursive: bool,
    auto_watch: bool,
    extensions: Vec<String>,
) -> Result<AddBindingResult, AppError> {
    let result = service::add_binding(&state, &path, recursive, auto_watch, &extensions)?;
    emit_state_changed(&app)?;
    Ok(result)
}

#[tauri::command(rename_all = "camelCase")]
pub fn update_binding(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    recursive: Option<bool>,
    auto_watch: Option<bool>,
    extensions: Option<Vec<String>>,
) -> Result<Binding, AppError> {
    let result =
        service::update_binding(&state, &id, recursive, auto_watch, extensions.as_deref())?;
    if result.unavailable {
        service::emit_binding_unavailable(&app, &result.id);
    }
    emit_state_changed(&app)?;
    Ok(result)
}

#[tauri::command]
pub fn remove_binding(
    app: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<RemoveBindingResult, AppError> {
    let result = service::remove_binding(&state, &id)?;
    emit_state_changed(&app)?;
    Ok(result)
}

#[tauri::command]
pub fn scan_binding(
    app: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<ReconcileSummary, AppError> {
    match service::scan_binding(&state, &id) {
        Ok(result) => {
            emit_state_changed(&app)?;
            Ok(result)
        }
        Err(err) => {
            if matches!(err, AppError::BindingUnavailable) {
                service::emit_binding_unavailable(&app, &id);
                let _ = emit_state_changed(&app);
            }
            Err(err)
        }
    }
}

#[tauri::command]
pub fn scan_all(app: AppHandle, state: State<AppState>) -> Result<ReconcileSummary, AppError> {
    let result = service::scan_all(&state)?;
    service::emit_unavailable_bindings(&state, &app);
    emit_state_changed(&app)?;
    Ok(result)
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<AppState>,
    extensions: Vec<String>,
) -> Result<Settings, AppError> {
    service::write_and_emit(&state, &app, |data| {
        service::update_settings(data, &extensions)
    })
}

fn emit_state_changed(app: &AppHandle) -> Result<(), AppError> {
    app.emit("state-changed", ())
        .map_err(|err| AppError::Io(io::Error::other(err.to_string())))
}

fn open_failed(err: impl ToString) -> AppError {
    AppError::OpenFailed(err.to_string())
}
