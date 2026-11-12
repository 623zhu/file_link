mod commands;
mod core;
mod error;
mod service;
mod watcher;

pub use core::model::{Binding, Data, Ignore, Link, Settings, Source};
pub use core::path::normalize;
pub use core::reconcile::{ReconcileSummary, reconcile, remove_gone, unbind};
pub use core::rule::{Rule, effective_rule, is_skipped_dir, is_temp_file, matches};
pub use core::scanner::scan;
pub use core::store::{LoadResult, load, save};
pub use error::AppError;
pub use service::{AppState, Snapshot, write, write_and_emit};

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind, TimezoneStrategy};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(log_plugin())
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
            watcher::start_app_worker(app.handle().clone());
            service::start_catch_up(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::import_files,
            commands::open_link,
            commands::reveal_link,
            commands::remove_links,
            commands::preview_binding,
            commands::add_binding,
            commands::update_binding,
            commands::remove_binding,
            commands::scan_binding,
            commands::scan_all,
            commands::update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let mut targets = vec![Target::new(TargetKind::LogDir {
        file_name: Some("file-link".into()),
    })];
    if cfg!(debug_assertions) {
        targets.push(Target::new(TargetKind::Stdout));
    }

    tauri_plugin_log::Builder::new()
        .level(log::LevelFilter::Info)
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .targets(targets)
        .build()
}
