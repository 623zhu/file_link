mod commands;
mod core;
mod error;
mod service;
mod watcher;

pub use core::model::{Binding, Data, Ignore, Link, Settings, Source};
pub use core::path::normalize;
pub use core::store::{LoadResult, load, save};
pub use error::AppError;
pub use service::{AppState, Snapshot, write, write_and_emit};

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            service::bootstrap(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::import_files,
            commands::open_link,
            commands::reveal_link,
            commands::remove_links,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
