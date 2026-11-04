use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;

use walkdir::WalkDir;

use super::path::normalize;
use super::rule::{self, Rule};

pub fn scan(
    path: impl AsRef<Path>,
    recursive: bool,
    rule: &Rule,
) -> io::Result<Vec<(String, String)>> {
    let path = path.as_ref();
    let meta = fs::metadata(path)?;
    if !meta.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "路径不是目录"));
    }
    if recursive {
        scan_recursive(path, rule)
    } else {
        scan_one_level(path, rule)
    }
}

fn scan_one_level(path: &Path, rule: &Rule) -> io::Result<Vec<(String, String)>> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    for entry in fs::read_dir(path)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                log::warn!("跳过无法读取的目录项: {err}");
                continue;
            }
        };
        let file_path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                log::warn!("跳过无法判断类型的项 {}: {err}", file_path.display());
                continue;
            }
        };
        if file_type.is_dir() {
            continue;
        }
        push_if_match(&file_path, rule, &mut results, &mut seen);
    }
    Ok(results)
}

fn scan_recursive(path: &Path, rule: &Rule) -> io::Result<Vec<(String, String)>> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    let walker = WalkDir::new(path).into_iter().filter_entry(|entry| {
        if entry.depth() == 0 {
            return true;
        }
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy();
            return !rule::is_skipped_dir(&name);
        }
        true
    });
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) if err.depth() == 0 => {
                return Err(io::Error::other(err.to_string()));
            }
            Err(err) => {
                log::warn!("跳过无法访问的路径: {err}");
                continue;
            }
        };
        if entry.file_type().is_dir() {
            continue;
        }
        push_if_match(entry.path(), rule, &mut results, &mut seen);
    }
    Ok(results)
}

fn push_if_match(
    path: &Path,
    rule: &Rule,
    results: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
) {
    if rule::is_temp_file(path) || !rule::matches(path, &rule.extensions) {
        return;
    }
    match normalize(path) {
        Ok((display, key)) => {
            if seen.insert(key.clone()) {
                results.push((display, key));
            }
        }
        Err(err) => {
            log::warn!("跳过无法规范化的路径 {}: {err}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn html_rule() -> Rule {
        Rule {
            extensions: vec![".html".to_string(), ".htm".to_string()],
        }
    }

    fn setup_tree() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        fs::write(root.join("root.html"), "ok").unwrap();
        fs::write(root.join("skip.txt"), "ok").unwrap();
        fs::write(root.join("~$lock.html"), "ok").unwrap();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("nested.html"), "ok").unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules").join("pkg.html"), "ok").unwrap();
        (dir, root)
    }

    fn names(results: &[(String, String)]) -> Vec<String> {
        let mut names: Vec<String> = results
            .iter()
            .map(|(display, _)| {
                Path::new(display)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn non_recursive_skips_nested_files_recursive_includes_them() {
        let (_dir, root) = setup_tree();
        let rule = html_rule();

        let shallow = scan(&root, false, &rule).unwrap();
        assert_eq!(names(&shallow), vec!["root.html".to_string()]);

        let deep = scan(&root, true, &rule).unwrap();
        assert_eq!(
            names(&deep),
            vec!["nested.html".to_string(), "root.html".to_string()]
        );
    }

    #[test]
    fn html_inside_node_modules_is_not_returned() {
        let (_dir, root) = setup_tree();
        let results = scan(&root, true, &html_rule()).unwrap();
        assert!(!names(&results).iter().any(|name| name == "pkg.html"));
    }

    #[test]
    fn missing_binding_dir_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such-dir");
        let err = scan(&missing, false, &html_rule()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn scan_results_have_unique_keys() {
        let (_dir, root) = setup_tree();
        let results = scan(&root, true, &html_rule()).unwrap();
        let unique: HashSet<_> = results.iter().map(|(_, key)| key).collect();
        assert_eq!(unique.len(), results.len());
        assert!(!results.is_empty());
    }
}
