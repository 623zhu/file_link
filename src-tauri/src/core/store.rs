use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::Local;

use super::model::{Data, Settings};

pub struct LoadResult {
    pub data: Data,
    pub backup_path: Option<PathBuf>,
}

pub fn load(path: impl AsRef<Path>) -> io::Result<LoadResult> {
    let path = path.as_ref();
    match fs::read(path) {
        Ok(bytes) => match serde_json::from_slice::<Data>(&bytes) {
            Ok(data) => Ok(LoadResult {
                data: migrate(data),
                backup_path: None,
            }),
            Err(_) => {
                let backup_path = broken_path(path);
                fs::rename(path, &backup_path)?;
                Ok(LoadResult {
                    data: Data::default(),
                    backup_path: Some(backup_path),
                })
            }
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(LoadResult {
            data: Data::default(),
            backup_path: None,
        }),
        Err(err) => Err(err),
    }
}

pub fn save(path: impl AsRef<Path>, data: &Data) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = tmp_path(path);
    {
        let mut file = fs::File::create(&tmp_path)?;
        serde_json::to_writer_pretty(&mut file, data).map_err(json_error)?;
        file.write_all(b"\n")?;
        file.flush()?;
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    Ok(())
}

fn migrate(mut data: Data) -> Data {
    loop {
        match data.version {
            0 => data.version = 1,
            1 => {
                let fallback = if data.settings.extensions.is_empty() {
                    Settings::html_extensions()
                } else {
                    data.settings.extensions.clone()
                };
                for binding in &mut data.bindings {
                    if binding.extensions.is_empty() {
                        binding.extensions = fallback.clone();
                    }
                }
                data.version = 2;
            }
            2 => {
                data.settings.extensions = Settings::html_extensions();
                for binding in &mut data.bindings {
                    if binding.extensions.is_empty() {
                        binding.extensions = Settings::html_extensions();
                    }
                }
                data.version = 3;
            }
            3 => {
                for link in &mut data.links {
                    if let Some(mtime) = super::path::try_file_modified_at(&link.path) {
                        link.modified_at = mtime;
                    }
                }
                data.version = 4;
            }
            _ => break,
        }
    }
    data
}

fn tmp_path(path: &Path) -> PathBuf {
    append_suffix(path, ".tmp")
}

fn broken_path(path: &Path) -> PathBuf {
    let stamp = Local::now().format("%Y%m%d%H%M%S");
    append_suffix(path, &format!(".broken-{stamp}"))
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

fn json_error(err: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{Link, Source};
    use chrono::TimeZone;
    use std::ffi::OsStr;

    fn sample_data() -> Data {
        Data {
            version: 4,
            settings: Default::default(),
            bindings: Vec::new(),
            links: vec![Link {
                id: "l1".to_string(),
                path: r"D:\tools\json.html".to_string(),
                key: r"d:\tools\json.html".to_string(),
                name: "json.html".to_string(),
                sources: vec![Source::Manual],
                modified_at: chrono::Local
                    .with_ymd_and_hms(2026, 11, 4, 15, 0, 0)
                    .unwrap(),
                missing: false,
                starred: false,
            }],
            ignored: Vec::new(),
        }
    }

    fn assert_no_tmp(dir: &Path) {
        for entry in fs::read_dir(dir).unwrap() {
            let name = entry.unwrap().file_name();
            assert!(
                !name.to_string_lossy().ends_with(".tmp"),
                "leftover tmp file: {name:?}"
            );
        }
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        let data = sample_data();

        save(&path, &data).unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(loaded.data, data);
        assert!(loaded.backup_path.is_none());
        assert_no_tmp(dir.path());
    }

    #[test]
    fn save_creates_missing_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("data.json");

        save(&path, &Data::default()).unwrap();

        assert!(path.is_file());
        assert_eq!(load(&path).unwrap().data, Data::default());
        assert_no_tmp(path.parent().unwrap());
    }

    #[test]
    fn load_missing_file_returns_default_without_creating_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");

        let loaded = load(&path).unwrap();

        assert_eq!(loaded.data, Data::default());
        assert!(loaded.backup_path.is_none());
        assert!(!path.exists());
    }

    #[test]
    fn load_corrupt_json_backs_up_and_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        let original = b"{not json";
        fs::write(&path, original).unwrap();

        let loaded = load(&path).unwrap();

        assert_eq!(loaded.data, Data::default());
        let backup_path = loaded.backup_path.expect("expected backup path");
        let backup_name = backup_path.file_name().and_then(OsStr::to_str).unwrap();
        assert!(backup_name.starts_with("data.json.broken-"));
        assert_eq!(fs::read(&backup_path).unwrap(), original);
        assert!(!path.exists());
    }

    #[test]
    fn migrate_steps_version_zero_to_current() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        let mut data = Data::default();
        data.version = 0;
        fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.data.version, 4);
        assert_eq!(loaded.data.settings, Data::default().settings);
    }

    #[test]
    fn migrate_copies_global_extensions_onto_bindings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        let json = serde_json::json!({
            "version": 1,
            "settings": { "extensions": [".md"] },
            "bindings": [{
                "id": "b1",
                "path": r"D:\notes",
                "key": r"d:\notes",
                "recursive": true,
                "autoWatch": true,
                "ruleId": null,
                "createdAt": "2026-11-04T15:00:00+08:00",
            }],
            "links": [],
            "ignored": []
        });
        fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.data.version, 4);
        assert_eq!(loaded.data.settings.extensions, [".html"]);
        assert_eq!(loaded.data.bindings[0].extensions, [".md"]);
    }

    #[test]
    fn migrate_resets_stale_global_extensions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        let json = serde_json::json!({
            "version": 2,
            "settings": { "extensions": [".html", ".htm", ".md", ".doc"] },
            "bindings": [
                {
                    "id": "empty",
                    "path": r"D:\pages",
                    "key": r"d:\pages",
                    "recursive": true,
                    "autoWatch": true,
                    "extensions": [],
                    "ruleId": null,
                    "createdAt": "2026-11-04T15:00:00+08:00",
                },
                {
                    "id": "custom",
                    "path": r"D:\notes",
                    "key": r"d:\notes",
                    "recursive": true,
                    "autoWatch": true,
                    "extensions": [".md"],
                    "ruleId": null,
                    "createdAt": "2026-11-04T15:00:00+08:00",
                }
            ],
            "links": [],
            "ignored": []
        });
        fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.data.version, 4);
        assert_eq!(loaded.data.settings.extensions, [".html"]);
        assert_eq!(loaded.data.bindings[0].extensions, [".html"]);
        assert_eq!(loaded.data.bindings[1].extensions, [".md"]);
    }

    #[test]
    fn migrate_replaces_added_at_with_file_modified_at() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("page.html");
        fs::write(&file, "ok").unwrap();
        let display = super::super::path::normalize(&file).unwrap().0;
        let stamp = "2020-01-01T00:00:00+08:00";
        let json = serde_json::json!({
            "version": 3,
            "settings": { "extensions": [".html"] },
            "bindings": [],
            "links": [{
                "id": "l1",
                "path": display,
                "key": display.to_lowercase(),
                "name": "page.html",
                "sources": [{ "type": "manual" }],
                "addedAt": stamp,
                "missing": false
            }],
            "ignored": []
        });
        let path = dir.path().join("data.json");
        fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.data.version, 4);
        let expected = super::super::path::file_modified_at(&loaded.data.links[0].path);
        assert_eq!(
            loaded.data.links[0].modified_at.timestamp(),
            expected.timestamp()
        );
        assert_ne!(
            loaded.data.links[0].modified_at.to_rfc3339(),
            "2020-01-01T00:00:00+08:00"
        );
    }
}
