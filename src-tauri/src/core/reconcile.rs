use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{DateTime, Local};
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
    let mut new_files = Vec::new();

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
            new_files.push((display, key, path::try_file_modified_at(display)));
        }
    }

    let gone = gone_links(data, &source, &scanned_keys, exists);

    let mut renamed_links = HashSet::new();
    let mut renamed_files = HashSet::new();
    for (link_index, file_index) in pair_renames(&data.links, &gone, &new_files) {
        apply_rename(&mut data.links[link_index], &new_files[file_index]);
        renamed_links.insert(link_index);
        renamed_files.insert(file_index);
    }

    let remove_ids: HashSet<String> = gone
        .iter()
        .filter(|index| !renamed_links.contains(*index))
        .map(|index| data.links[*index].id.clone())
        .collect();
    summary.removed = remove_ids.len();
    data.links.retain(|link| !remove_ids.contains(&link.id));

    for (file_index, (display, key, mtime)) in new_files.into_iter().enumerate() {
        if renamed_files.contains(&file_index) {
            continue;
        }
        data.links.push(Link {
            id: Uuid::new_v4().to_string(),
            path: display.clone(),
            key: key.clone(),
            name: file_name(display),
            sources: vec![source.clone()],
            modified_at: mtime.unwrap_or_else(Local::now),
            missing: false,
            starred: false,
        });
        summary.added += 1;
    }

    summary
}

/// Moves only the link `link_id` to its renamed file, using the same pairing as
/// `reconcile`. Other new files are not imported and other links are untouched.
pub fn relocate(
    data: &mut Data,
    binding_id: &str,
    files: &[(String, String)],
    exists: impl Fn(&str) -> bool,
    link_id: &str,
) -> bool {
    let source = Source::Binding {
        binding_id: binding_id.to_string(),
    };
    let scanned_keys: HashSet<&str> = files.iter().map(|(_, key)| key.as_str()).collect();
    let new_files: Vec<_> = files
        .iter()
        .filter(|(_, key)| {
            !is_ignored(data, binding_id, key) && !data.links.iter().any(|link| link.key == *key)
        })
        .map(|(display, key)| (display, key, path::try_file_modified_at(display)))
        .collect();
    let gone = gone_links(data, &source, &scanned_keys, exists);

    let Some((link_index, file_index)) = pair_renames(&data.links, &gone, &new_files)
        .into_iter()
        .find(|(index, _)| data.links[*index].id == link_id)
    else {
        return false;
    };
    apply_rename(&mut data.links[link_index], &new_files[file_index]);
    true
}

fn gone_links(
    data: &Data,
    source: &Source,
    scanned_keys: &HashSet<&str>,
    exists: impl Fn(&str) -> bool,
) -> Vec<usize> {
    data.links
        .iter()
        .enumerate()
        .filter(|(_, link)| {
            link.has_source(source)
                && !scanned_keys.contains(link.key.as_str())
                && !exists(&link.path)
        })
        .map(|(index, _)| index)
        .collect()
}

fn apply_rename(
    link: &mut Link,
    (display, key, mtime): &(&String, &String, Option<DateTime<Local>>),
) {
    link.path = (*display).clone();
    link.key = (*key).clone();
    link.name = file_name(display);
    if let Some(mtime) = mtime {
        link.modified_at = *mtime;
    }
    link.missing = false;
}

/// NTFS keeps the modified time on same-volume renames and moves, so a gone
/// link and a new file sharing it exactly are treated as the same file.
/// Times claimed by more than one link or file are left unpaired.
fn pair_renames(
    links: &[Link],
    gone: &[usize],
    new_files: &[(&String, &String, Option<DateTime<Local>>)],
) -> Vec<(usize, usize)> {
    let mut by_time: HashMap<DateTime<Local>, (Vec<usize>, Vec<usize>)> = HashMap::new();
    for &index in gone {
        by_time
            .entry(links[index].modified_at)
            .or_default()
            .0
            .push(index);
    }
    for (index, (_, _, mtime)) in new_files.iter().enumerate() {
        if let Some(entry) = mtime.and_then(|mtime| by_time.get_mut(&mtime)) {
            entry.1.push(index);
        }
    }
    by_time
        .into_values()
        .filter_map(
            |(links, files)| match (links.as_slice(), files.as_slice()) {
                ([link], [file]) => Some((*link, *file)),
                _ => None,
            },
        )
        .collect()
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
            starred: false,
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
    fn reconcile_keeps_starred_on_existing_link() {
        let files = [file("starred.html")];
        let mut data = Data::default();
        let mut existing = link("starred.html", vec![binding_source(BINDING)], false);
        existing.starred = true;
        data.links.push(existing);

        reconcile(&mut data, BINDING, &files, always_exists);

        assert_eq!(data.links.len(), 1);
        assert!(data.links[0].starred);
    }

    fn path_exists(path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    #[test]
    fn renamed_file_keeps_link_and_star() {
        let dir = tempfile::tempdir().unwrap();
        let old_path = dir.path().join("old.html");
        fs::write(&old_path, "ok").unwrap();
        let mut data = Data::default();
        reconcile(
            &mut data,
            BINDING,
            &[crate::core::path::normalize(&old_path).unwrap()],
            path_exists,
        );
        data.links[0].starred = true;
        let id = data.links[0].id.clone();

        fs::create_dir(dir.path().join("sub")).unwrap();
        let new_path = dir.path().join("sub").join("new.html");
        fs::rename(&old_path, &new_path).unwrap();
        let (display, key) = crate::core::path::normalize(&new_path).unwrap();
        let summary = reconcile(
            &mut data,
            BINDING,
            &[(display.clone(), key.clone())],
            path_exists,
        );

        assert_eq!(summary, ReconcileSummary::default());
        assert_eq!(data.links.len(), 1);
        let link = &data.links[0];
        assert_eq!(link.id, id);
        assert!(link.starred);
        assert_eq!(link.path, display);
        assert_eq!(link.key, key);
        assert_eq!(link.name, "new.html");
    }

    #[test]
    fn relocate_moves_only_the_requested_link() {
        let dir = tempfile::tempdir().unwrap();
        let old_path = dir.path().join("old.html");
        fs::write(&old_path, "ok").unwrap();
        let mut data = Data::default();
        reconcile(
            &mut data,
            BINDING,
            &[crate::core::path::normalize(&old_path).unwrap()],
            path_exists,
        );
        data.links[0].starred = true;
        let id = data.links[0].id.clone();

        let new_path = dir.path().join("new.html");
        fs::rename(&old_path, &new_path).unwrap();
        let other_path = dir.path().join("other.html");
        fs::write(&other_path, "other").unwrap();
        let files = [
            crate::core::path::normalize(&new_path).unwrap(),
            crate::core::path::normalize(&other_path).unwrap(),
        ];

        assert!(!relocate(
            &mut data,
            BINDING,
            &files,
            path_exists,
            "unknown"
        ));
        assert!(relocate(&mut data, BINDING, &files, path_exists, &id));

        assert_eq!(data.links.len(), 1);
        let link = &data.links[0];
        assert_eq!(link.id, id);
        assert!(link.starred);
        assert_eq!(link.path, files[0].0);
        assert_eq!(link.name, "new.html");
    }

    #[test]
    fn ambiguous_rename_is_treated_as_remove_and_add() {
        let dir = tempfile::tempdir().unwrap();
        let new_path = dir.path().join("new.html");
        fs::write(&new_path, "ok").unwrap();
        let (display, key) = crate::core::path::normalize(&new_path).unwrap();
        let mtime = crate::core::path::file_modified_at(&display);
        let mut data = Data::default();
        for name in ["a.html", "b.html"] {
            let mut gone = link(name, vec![binding_source(BINDING)], false);
            gone.modified_at = mtime;
            gone.starred = true;
            data.links.push(gone);
        }

        let summary = reconcile(&mut data, BINDING, &[(display, key)], path_exists);

        assert_eq!(
            summary,
            ReconcileSummary {
                added: 1,
                restored: 0,
                removed: 2
            }
        );
        assert_eq!(data.links.len(), 1);
        assert_eq!(data.links[0].name, "new.html");
        assert!(!data.links[0].starred);
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
