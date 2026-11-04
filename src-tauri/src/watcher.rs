use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::core::model::Binding;
use crate::error::AppError;
use crate::service::{self, AppState};

const DEBOUNCE_WINDOW: Duration = Duration::from_millis(1500);
const COALESCE_WINDOW: Duration = Duration::from_millis(500);

type FolderDebouncer = Debouncer<RecommendedWatcher, RecommendedCache>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutoImportedPayload {
    binding_id: String,
    added: usize,
}

pub struct WatcherManager {
    watchers: Mutex<HashMap<String, FolderDebouncer>>,
    tx: Sender<String>,
    rx: Mutex<Option<Receiver<String>>>,
}

impl WatcherManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            watchers: Mutex::new(HashMap::new()),
            tx,
            rx: Mutex::new(Some(rx)),
        }
    }

    pub fn start(&self, binding: &Binding) -> Result<(), AppError> {
        self.stop(&binding.id);
        if !binding.auto_watch {
            return Ok(());
        }

        let tx = self.tx.clone();
        let binding_id = binding.id.clone();
        let mut debouncer =
            new_debouncer(DEBOUNCE_WINDOW, None, move |result: DebounceEventResult| {
                if let Err(errors) = &result {
                    for error in errors {
                        log::warn!("folder watcher error for {binding_id}: {error}");
                    }
                }
                if tx.send(binding_id.clone()).is_err() {
                    log::warn!("folder watcher event dropped; worker stopped");
                }
            })
            .map_err(watch_error)?;

        let mode = if binding.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        debouncer
            .watch(Path::new(&binding.path), mode)
            .map_err(watch_error)?;

        lock(&self.watchers)?.insert(binding.id.clone(), debouncer);
        log::info!(
            "started folder watcher for binding {} at {}",
            binding.id,
            binding.path
        );
        Ok(())
    }

    pub fn stop(&self, binding_id: &str) {
        match lock(&self.watchers) {
            Ok(mut watchers) => {
                if watchers.remove(binding_id).is_some() {
                    log::info!("stopped folder watcher for binding {binding_id}");
                }
            }
            Err(err) => log::error!("failed to stop folder watcher for {binding_id}: {err}"),
        }
    }

    pub fn restart(&self, binding: &Binding) -> Result<(), AppError> {
        self.start(binding)
    }

    pub(crate) fn schedule_reconcile(&self, binding_id: &str, delay: Duration) {
        let tx = self.tx.clone();
        let id = binding_id.to_string();
        if let Err(err) = std::thread::Builder::new()
            .name("folder-watcher-retry".into())
            .spawn({
                let id = id.clone();
                move || {
                    std::thread::sleep(delay);
                    if tx.send(id).is_err() {
                        log::warn!("scheduled reconcile dropped; worker stopped");
                    }
                }
            })
        {
            log::error!("failed to schedule folder reconcile for {id}: {err}");
        }
    }

    #[cfg(test)]
    pub fn is_watching(&self, binding_id: &str) -> bool {
        lock(&self.watchers)
            .map(|watchers| watchers.contains_key(binding_id))
            .unwrap_or(false)
    }

    pub(crate) fn take_receiver(&self) -> Option<Receiver<String>> {
        lock(&self.rx).ok().and_then(|mut rx| rx.take())
    }
}

pub fn start_app_worker<R: Runtime>(app: AppHandle<R>) {
    let Some(rx) = app.state::<AppState>().watcher.take_receiver() else {
        log::warn!("folder watcher worker already started");
        return;
    };
    spawn_worker(rx, move |id| handle_watch_event(&app, &id));
}

pub(crate) fn spawn_worker(rx: Receiver<String>, on_binding: impl Fn(String) + Send + 'static) {
    if let Err(err) = std::thread::Builder::new()
        .name("folder-watcher".into())
        .spawn(move || {
            while let Some(ids) = next_batch(&rx, COALESCE_WINDOW) {
                for id in ids {
                    on_binding(id);
                }
            }
            log::info!("folder watcher worker stopped");
        })
    {
        log::error!("failed to spawn folder watcher worker: {err}");
    }
}

fn handle_watch_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let state = app.state::<AppState>();
    match service::reconcile_binding(&state, id) {
        Ok(summary) => {
            if summary.added > 0 {
                let payload = AutoImportedPayload {
                    binding_id: id.to_string(),
                    added: summary.added,
                };
                if let Err(err) = app.emit("auto-imported", payload) {
                    log::warn!("failed to emit auto-imported: {err}");
                }
            }
            if let Err(err) = app.emit("state-changed", ()) {
                log::warn!("failed to emit state-changed: {err}");
            }
        }
        Err(AppError::NotFound) => {
            log::debug!("skip auto reconcile for removed binding {id}");
        }
        Err(AppError::BindingUnavailable) => {
            service::emit_binding_unavailable(app, id);
            if let Err(err) = app.emit("state-changed", ()) {
                log::warn!("failed to emit state-changed: {err}");
            }
        }
        Err(err) => {
            log::warn!("auto reconcile failed for {id}: {err}");
        }
    }
}

fn next_batch(rx: &Receiver<String>, window: Duration) -> Option<HashSet<String>> {
    let first = rx.recv().ok()?;
    let mut ids = HashSet::new();
    ids.insert(first);
    let deadline = Instant::now() + window;
    while Instant::now() < deadline {
        let timeout = deadline.saturating_duration_since(Instant::now());
        if timeout.is_zero() {
            break;
        }
        match rx.recv_timeout(timeout) {
            Ok(id) => {
                ids.insert(id);
            }
            Err(RecvTimeoutError::Timeout) => break,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Some(ids)
}

fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, AppError> {
    mutex
        .lock()
        .map_err(|_| io::Error::other("folder watcher lock poisoned").into())
}

fn watch_error(err: impl ToString) -> AppError {
    io::Error::other(err.to_string()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::Data;
    use crate::core::store;
    use crate::service::{add_binding, remove_binding, snapshot, update_binding};
    use chrono::Local;
    use std::fs;
    use std::sync::Arc;
    use std::thread;

    fn sample_binding(dir: &Path, auto_watch: bool) -> Binding {
        Binding {
            id: "bind-1".to_string(),
            path: dir.to_string_lossy().into_owned(),
            key: dir.to_string_lossy().to_lowercase(),
            recursive: false,
            auto_watch,
            rule_id: None,
            created_at: Local::now(),
            unavailable: false,
        }
    }

    fn temp_state() -> (tempfile::TempDir, tempfile::TempDir, AppState) {
        let files = tempfile::tempdir().unwrap();
        let data_dir = tempfile::tempdir().unwrap();
        let path = data_dir.path().join("data.json");
        let data = Data::default();
        store::save(&path, &data).unwrap();
        (files, data_dir, AppState::new(data, path, None))
    }

    fn write_html(dir: &Path, name: &str) {
        fs::write(dir.join(name), "<html></html>").unwrap();
    }

    fn wait_for_link(state: &AppState, name: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if snapshot(state)
                .unwrap()
                .links
                .iter()
                .any(|link| link.name == name)
            {
                return true;
            }
            thread::sleep(Duration::from_millis(100));
        }
        false
    }

    fn start_test_worker(state: &Arc<AppState>) {
        let rx = state.watcher.take_receiver().expect("watcher receiver");
        let state = Arc::clone(state);
        spawn_worker(rx, move |id| {
            let _ = service::reconcile_binding(&state, &id);
        });
    }

    #[test]
    fn start_watches_only_when_auto_watch_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let manager = WatcherManager::new();
        let mut binding = sample_binding(dir.path(), false);

        manager.start(&binding).unwrap();
        assert!(!manager.is_watching(&binding.id));

        binding.auto_watch = true;
        manager.start(&binding).unwrap();
        assert!(manager.is_watching(&binding.id));

        manager.stop(&binding.id);
        assert!(!manager.is_watching(&binding.id));
    }

    #[test]
    fn restart_rebuilds_watcher_for_updated_binding() {
        let dir = tempfile::tempdir().unwrap();
        let manager = WatcherManager::new();
        let mut binding = sample_binding(dir.path(), true);
        manager.start(&binding).unwrap();
        assert!(manager.is_watching(&binding.id));

        binding.recursive = true;
        manager.restart(&binding).unwrap();
        assert!(manager.is_watching(&binding.id));

        binding.auto_watch = false;
        manager.restart(&binding).unwrap();
        assert!(!manager.is_watching(&binding.id));
    }

    #[test]
    fn next_batch_merges_duplicate_binding_ids() {
        let (tx, rx) = mpsc::channel();
        tx.send("a".to_string()).unwrap();
        tx.send("a".to_string()).unwrap();
        tx.send("b".to_string()).unwrap();
        drop(tx);

        let batch = next_batch(&rx, Duration::from_millis(20)).unwrap();
        assert_eq!(batch.len(), 2);
        assert!(batch.contains("a"));
        assert!(batch.contains("b"));
    }

    #[test]
    #[ignore]
    fn watcher_imports_new_html_within_a_few_seconds() {
        let (files, _data_dir, state) = temp_state();
        write_html(files.path(), "existing.html");
        let state = Arc::new(state);
        start_test_worker(&state);

        let added = add_binding(&state, &files.path().to_string_lossy(), false, true).unwrap();
        assert!(state.watcher.is_watching(&added.binding.id));
        thread::sleep(Duration::from_millis(200));

        write_html(files.path(), "new.html");
        assert!(
            wait_for_link(&state, "new.html", Duration::from_secs(8)),
            "expected new.html to be imported by the folder watcher"
        );
    }

    #[test]
    #[ignore]
    fn disabling_auto_watch_does_not_auto_import() {
        let (files, _data_dir, state) = temp_state();
        write_html(files.path(), "existing.html");
        let state = Arc::new(state);
        start_test_worker(&state);

        let added = add_binding(&state, &files.path().to_string_lossy(), false, true).unwrap();
        update_binding(&state, &added.binding.id, None, Some(false)).unwrap();
        assert!(!state.watcher.is_watching(&added.binding.id));

        write_html(files.path(), "manual.html");
        thread::sleep(Duration::from_secs(4));
        assert!(
            !snapshot(&state)
                .unwrap()
                .links
                .iter()
                .any(|link| link.name == "manual.html"),
            "file copied after auto-watch was disabled should not be imported"
        );
    }

    #[test]
    #[ignore]
    fn removing_binding_stops_the_watcher() {
        let (files, _data_dir, state) = temp_state();
        write_html(files.path(), "existing.html");
        let state = Arc::new(state);
        start_test_worker(&state);

        let added = add_binding(&state, &files.path().to_string_lossy(), false, true).unwrap();
        let binding_id = added.binding.id.clone();
        assert!(state.watcher.is_watching(&binding_id));

        remove_binding(&state, &binding_id).unwrap();
        assert!(
            !state.watcher.is_watching(&binding_id),
            "debouncer should stop after the binding is removed"
        );

        write_html(files.path(), "after-unbind.html");
        thread::sleep(Duration::from_secs(4));
        assert!(snapshot(&state).unwrap().links.is_empty());
    }

    fn has_link(state: &AppState, name: &str) -> bool {
        snapshot(state)
            .unwrap()
            .links
            .iter()
            .any(|link| link.name == name)
    }

    #[test]
    #[ignore]
    fn growing_html_is_imported_only_after_write_finishes() {
        use std::io::Write;

        let (files, _data_dir, state) = temp_state();
        write_html(files.path(), "existing.html");
        let state = Arc::new(state);
        start_test_worker(&state);
        add_binding(&state, &files.path().to_string_lossy(), false, true).unwrap();
        thread::sleep(Duration::from_millis(200));

        let growing = files.path().join("growing.html");
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let writer = {
            let growing = growing.clone();
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                let mut file = fs::File::create(&growing).unwrap();
                file.write_all(b"<html>\n").unwrap();
                while !stop.load(std::sync::atomic::Ordering::SeqCst) {
                    writeln!(file, "<p>{}</p>", file.metadata().unwrap().len()).unwrap();
                    file.flush().unwrap();
                    thread::sleep(Duration::from_millis(50));
                }
                file.write_all(b"</html>\n").unwrap();
                file.flush().unwrap();
            })
        };

        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            assert!(
                !has_link(&state, "growing.html"),
                "in-progress write should not create a link"
            );
            thread::sleep(Duration::from_millis(100));
        }
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        writer.join().unwrap();

        assert!(
            wait_for_link(&state, "growing.html", Duration::from_secs(10)),
            "growing.html should be imported after the write finishes"
        );
    }

    #[test]
    #[ignore]
    fn chrome_download_temp_file_is_not_imported() {
        let (files, _data_dir, state) = temp_state();
        let state = Arc::new(state);
        start_test_worker(&state);
        add_binding(&state, &files.path().to_string_lossy(), false, true).unwrap();
        thread::sleep(Duration::from_millis(200));

        let temp = files.path().join("page.html.crdownload");
        fs::write(&temp, "<html>partial").unwrap();
        thread::sleep(Duration::from_secs(4));
        assert!(
            snapshot(&state).unwrap().links.is_empty(),
            ".crdownload should not become a link"
        );

        fs::rename(&temp, files.path().join("page.html")).unwrap();
        assert!(
            wait_for_link(&state, "page.html", Duration::from_secs(8)),
            "final HTML should be imported after the download finishes"
        );
        assert!(
            snapshot(&state)
                .unwrap()
                .links
                .iter()
                .all(|link| !link.name.contains("crdownload"))
        );
    }
}
