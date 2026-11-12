use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;
use uuid::Uuid;

use super::model::{Data, Link, Source};
use super::path;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileSummary {
    pub added: usize,
    pub restored: usize,
    pub removed: usize,
}

pub fn reconcile(
    data: &mut Data,
    binding_id: &str,
    files: &[(String, String)],
    exists: impl Fn(&str) -> bool,
) -> ReconcileSummary {
    let mut summary = ReconcileSummary::default();
    let source = Source::Binding {
        binding_id: binding_id.to_string(),
    };
    let scanned_keys: HashSet<&str> = files.iter().map(|(_, key)| key.as_str()).collect();

    for (display, key) in files {
        if is_ignored(data, binding_id, key) {
            continue;
        }
        if let Some(link) = data.links.iter_mut().find(|link| link.key == *key) {
            link.add_source(source.clone());
            if let Some(mtime) = path::try_file_modified_at(display) {
                link.modified_at = mtime;
            }
            if link.missing {
                link.missing = false;
                summary.restored += 1;
            }
        } else {
            data.links.push(Link {
                id: Uuid::new_v4().to_string(),
                path: display.clone(),
                key: key.clone(),
                name: file_name(display),
                sources: vec![source.clone()],
                modified_at: path::file_modified_at(display),
                missing: false,
            });
            summary.added += 1;
        }
    }

    data.links.retain(|link| {
        if !link.has_source(&source) {
            return true;
        }
        if scanned_keys.contains(link.key.as_str()) || exists(&link.path) {
            return true;
        }
        summary.removed += 1;
        false
    });

    summary
}

pub fn remove_gone(data: &mut Data, exists: impl Fn(&str) -> bool) -> usize {
    let unavailable: HashSet<String> = data
        .bindings
        .iter()
        .filter(|binding| binding.unavailable)
        .map(|binding| binding.id.clone())
        .collect();
    for link in &mut data.links {
        if exists(&link.path) {
            link.missing = false;
        }
    }
    let mut removed = 0;
    data.links.retain(|link| {
        if exists(&link.path) {
            return true;
        }
        let keep = link.sources.iter().any(|source| match source {
            Source::Binding { binding_id } => unavailable.contains(binding_id),
            Source::Manual => false,
        });
        if keep {
            true
        } else {
            removed += 1;
            false
        }
    });
    removed
}

pub fn unbind(data: &mut Data, binding_id: &str) -> usize {
    let before = data.links.len();
    for link in &mut data.links {
        link.remove_binding_source(binding_id);
    }
    data.links.retain(|link| !link.sources.is_empty());
    data.ignored
        .retain(|ignore| ignore.binding_id != binding_id);
    before - data.links.len()
}

fn is_ignored(data: &Data, binding_id: &str, key: &str) -> bool {
    data.ignored
        .iter()
        .any(|ignore| ignore.binding_id == binding_id && ignore.key == key)
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::Ignore;
    use chrono::{Local, TimeZone};
    use std::collections::HashSet;
    use std::fs;

    const BINDING: &str = "b1";
    const OTHER: &str = "b2";

    fn sample_time() -> chrono::DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 4, 15, 0, 0).unwrap()
    }

    fn file(name: &str) -> (String, String) {
        let display = format!(r"D:\bind\{name}");
        let key = display.to_lowercase();
        (display, key)
    }

    fn link(name: &str, sources: Vec<Source>, missing: bool) -> Link {
        let (path, key) = file(name);
        Link {
            id: name.to_string(),
            path,
            key,
            name: name.to_string(),
            sources,
            modified_at: sample_time(),
            missing,
        }
    }

    fn binding_source(id: &str) -> Source {
        Source::Binding {
            binding_id: id.to_string(),
        }
    }

    fn exists_among(paths: &[String]) -> impl Fn(&str) -> bool {
        let set: HashSet<String> = paths.iter().cloned().collect();
        move |path: &str| set.contains(path)
    }

    fn always_missing(_: &str) -> bool {
        false
    }

    fn always_exists(_: &str) -> bool {
        true
    }

    #[test]
    fn new_files_are_added_once() {
        let mut data = Data::default();
        let files = [file("a.html"), file("b.html")];

        let first = reconcile(&mut data, BINDING, &files, always_exists);
        let second = reconcile(&mut data, BINDING, &files, always_exists);

        assert_eq!(
            first,
            ReconcileSummary {
                added: 2,
                restored: 0,
                removed: 0
            }
        );
        assert_eq!(second, ReconcileSummary::default());
        assert_eq!(data.links.len(), 2);
        assert!(
            data.links
                .iter()
                .all(|link| link.sources == [binding_source(BINDING)])
        );
    }

    #[test]
    fn ignored_files_are_not_added() {
        let (display, key) = file("ignored.html");
        let mut data = Data::default();
        data.ignored.push(Ignore {
            binding_id: BINDING.to_string(),
            key: key.clone(),
        });

        let summary = reconcile(&mut data, BINDING, &[(display, key)], always_exists);

        assert_eq!(summary, ReconcileSummary::default());
        assert!(data.links.is_empty());
    }

    #[test]
    fn gone_files_are_removed_and_reimported_when_they_return() {
        let mut data = Data::default();
        let files = [file("gone.html")];
        reconcile(&mut data, BINDING, &files, always_exists);

        let removed = reconcile(&mut data, BINDING, &[], always_missing);
        assert_eq!(
            removed,
            ReconcileSummary {
                added: 0,
                restored: 0,
                removed: 1
            }
        );
        assert!(data.links.is_empty());

        let added = reconcile(&mut data, BINDING, &files, always_exists);
        assert_eq!(
            added,
            ReconcileSummary {
                added: 1,
                restored: 0,
                removed: 0
            }
        );
        assert_eq!(data.links.len(), 1);
        assert!(!data.links[0].missing);
    }

    #[test]
    fn old_missing_links_are_restored_when_the_file_returns() {
        let mut data = Data::default();
        data.links
            .push(link("gone.html", vec![binding_source(BINDING)], true));
        let files = [file("gone.html")];

        let restored = reconcile(&mut data, BINDING, &files, always_exists);
        assert_eq!(
            restored,
            ReconcileSummary {
                added: 0,
                restored: 1,
                removed: 0
            }
        );
        assert_eq!(data.links.len(), 1);
        assert!(!data.links[0].missing);
    }

    #[test]
    fn existing_files_not_in_scan_are_kept_without_missing() {
        let mut data = Data::default();
        data.links
            .push(link("old.html", vec![binding_source(BINDING)], false));

        let summary = reconcile(&mut data, BINDING, &[], always_exists);

        assert_eq!(summary, ReconcileSummary::default());
        assert_eq!(data.links.len(), 1);
        assert!(!data.links[0].missing);
        assert_eq!(data.links[0].sources, [binding_source(BINDING)]);
    }

    #[test]
    fn manual_link_merges_binding_source_for_the_same_file() {
        let mut data = Data::default();
        data.links
            .push(link("shared.html", vec![Source::Manual], false));
        let files = [file("shared.html")];

        let summary = reconcile(&mut data, BINDING, &files, always_exists);

        assert_eq!(summary.added, 0);
        assert_eq!(data.links.len(), 1);
        assert_eq!(
            data.links[0].sources,
            [Source::Manual, binding_source(BINDING)]
        );
    }

    #[test]
    fn unbind_removes_binding_only_links_and_clears_ignores() {
        let mut data = Data::default();
        data.links
            .push(link("auto.html", vec![binding_source(BINDING)], false));
        data.links.push(link(
            "both.html",
            vec![Source::Manual, binding_source(BINDING)],
            false,
        ));
        data.links
            .push(link("other.html", vec![binding_source(OTHER)], false));
        data.ignored.extend([
            Ignore {
                binding_id: BINDING.to_string(),
                key: file("auto.html").1,
            },
            Ignore {
                binding_id: OTHER.to_string(),
                key: file("other.html").1,
            },
        ]);

        let removed = unbind(&mut data, BINDING);

        assert_eq!(removed, 1);
        assert_eq!(data.links.len(), 2);
        let both = data
            .links
            .iter()
            .find(|link| link.name == "both.html")
            .unwrap();
        assert_eq!(both.sources, [Source::Manual]);
        assert!(
            data.links
                .iter()
                .any(|link| link.name == "other.html" && link.sources == [binding_source(OTHER)])
        );
        assert_eq!(data.ignored.len(), 1);
        assert_eq!(data.ignored[0].binding_id, OTHER);
    }

    #[test]
    fn remove_gone_deletes_missing_manual_links_and_clears_restored_flag() {
        let mut data = Data::default();
        data.links
            .push(link("present.html", vec![Source::Manual], true));
        data.links
            .push(link("absent.html", vec![Source::Manual], false));
        let present = data.links[0].path.clone();

        let removed = remove_gone(&mut data, exists_among(std::slice::from_ref(&present)));

        assert_eq!(removed, 1);
        assert_eq!(data.links.len(), 1);
        assert_eq!(data.links[0].name, "present.html");
        assert!(!data.links[0].missing);
    }

    #[test]
    fn remove_gone_keeps_links_when_binding_is_unavailable() {
        use crate::core::model::Binding;
        let mut data = Data::default();
        data.bindings.push(Binding {
            id: BINDING.to_string(),
            path: r"D:\bind".to_string(),
            key: r"d:\bind".to_string(),
            recursive: false,
            auto_watch: true,
            extensions: crate::core::model::Settings::html_extensions(),
            rule_id: None,
            created_at: sample_time(),
            unavailable: true,
        });
        data.links
            .push(link("offline.html", vec![binding_source(BINDING)], false));
        data.links
            .push(link("manual-gone.html", vec![Source::Manual], false));

        let removed = remove_gone(&mut data, always_missing);

        assert_eq!(removed, 1);
        assert_eq!(data.links.len(), 1);
        assert_eq!(data.links[0].name, "offline.html");
    }

    #[test]
    fn new_and_existing_links_use_file_modified_at() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("page.html");
        fs::write(&file_path, "ok").unwrap();
        let (display, key) = crate::core::path::normalize(&file_path).unwrap();
        let mut data = Data::default();

        reconcile(
            &mut data,
            BINDING,
            &[(display.clone(), key.clone())],
            always_exists,
        );

        let expected = crate::core::path::file_modified_at(&display);
        assert_eq!(data.links[0].modified_at.timestamp(), expected.timestamp());

        data.links[0].modified_at = sample_time();
        fs::write(&file_path, "updated").unwrap();
        reconcile(&mut data, BINDING, &[(display.clone(), key)], always_exists);
        let updated = crate::core::path::file_modified_at(&display);
        assert_eq!(data.links[0].modified_at.timestamp(), updated.timestamp());
        assert_ne!(data.links[0].modified_at, sample_time());
    }
}
