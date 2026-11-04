use std::collections::HashSet;
use std::path::Path;

use chrono::Local;
use serde::Serialize;
use uuid::Uuid;

use super::model::{Data, Link, Source};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileSummary {
    pub added: usize,
    pub restored: usize,
    pub missing: usize,
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
                added_at: Local::now(),
                missing: false,
            });
            summary.added += 1;
        }
    }

    for link in data.links.iter_mut() {
        if !link.has_source(&source) {
            continue;
        }
        if scanned_keys.contains(link.key.as_str()) || exists(&link.path) {
            continue;
        }
        if !link.missing {
            link.missing = true;
            summary.missing += 1;
        }
    }

    summary
}

pub fn refresh_missing(data: &mut Data, exists: impl Fn(&str) -> bool) -> usize {
    let mut changed = 0;
    for link in &mut data.links {
        let missing = !exists(&link.path);
        if link.missing != missing {
            link.missing = missing;
            changed += 1;
        }
    }
    changed
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
    use chrono::TimeZone;
    use std::collections::HashSet;

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
            added_at: sample_time(),
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
                missing: 0
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
    fn gone_files_are_marked_missing_and_restored_when_they_return() {
        let mut data = Data::default();
        let files = [file("gone.html")];
        reconcile(&mut data, BINDING, &files, always_exists);

        let missing = reconcile(&mut data, BINDING, &[], always_missing);
        assert_eq!(
            missing,
            ReconcileSummary {
                added: 0,
                restored: 0,
                missing: 1
            }
        );
        assert!(data.links[0].missing);

        let restored = reconcile(&mut data, BINDING, &files, always_exists);
        assert_eq!(
            restored,
            ReconcileSummary {
                added: 0,
                restored: 1,
                missing: 0
            }
        );
        assert!(!data.links[0].missing);
        assert_eq!(data.links.len(), 1);
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
    fn refresh_missing_updates_manual_links() {
        let mut data = Data::default();
        data.links
            .push(link("present.html", vec![Source::Manual], true));
        data.links
            .push(link("absent.html", vec![Source::Manual], false));
        let present = data.links[0].path.clone();

        let changed = refresh_missing(&mut data, exists_among(std::slice::from_ref(&present)));

        assert_eq!(changed, 2);
        let present_link = data
            .links
            .iter()
            .find(|link| link.name == "present.html")
            .unwrap();
        let absent_link = data
            .links
            .iter()
            .find(|link| link.name == "absent.html")
            .unwrap();
        assert!(!present_link.missing);
        assert!(absent_link.missing);
    }
}
