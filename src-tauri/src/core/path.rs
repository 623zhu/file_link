use std::io;
use std::path::Path;

pub fn normalize(path: impl AsRef<Path>) -> io::Result<(String, String)> {
    let canonical = dunce::canonicalize(path)?;
    let display = to_display(&canonical);
    let key = display.to_lowercase();
    Ok((display, key))
}

fn to_display(path: &Path) -> String {
    let unified = path.to_string_lossy().replace('/', "\\");
    let trimmed = unified.trim_end_matches('\\');
    if is_drive_root(trimmed) {
        format!("{trimmed}\\")
    } else {
        trimmed.to_string()
    }
}

fn is_drive_root(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_file() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Test.HTML");
        fs::write(&file, "ok").unwrap();
        (dir, file)
    }

    #[test]
    fn same_path_variants_share_a_key() {
        let (dir, file) = temp_file();
        let original = file.to_string_lossy().into_owned();
        let mixed = original.replace('\\', "/");
        let trailing_dir = format!(
            "{}\\",
            dir.path().to_string_lossy().trim_end_matches(['\\', '/'])
        );
        let trailing_slash = format!(
            "{}/",
            dir.path()
                .to_string_lossy()
                .trim_end_matches(['\\', '/'])
                .replace('\\', "/")
        );

        let (_, file_key) = normalize(&original).unwrap();
        let (_, case_key) = normalize(original.to_uppercase()).unwrap();
        let (_, mixed_key) = normalize(&mixed).unwrap();
        let (_, dir_key) = normalize(dir.path()).unwrap();
        let (_, trailing_key) = normalize(&trailing_dir).unwrap();
        let (_, slash_key) = normalize(&trailing_slash).unwrap();

        assert_eq!(file_key, case_key);
        assert_eq!(file_key, mixed_key);
        assert_eq!(dir_key, trailing_key);
        assert_eq!(dir_key, slash_key);
    }

    #[test]
    fn display_has_no_verbatim_prefix() {
        let (_dir, file) = temp_file();
        let (display, key) = normalize(&file).unwrap();
        assert!(!display.starts_with(r"\\?\"));
        assert!(!key.starts_with(r"\\?\"));
        assert_eq!(key, display.to_lowercase());
    }

    #[test]
    fn relative_path_becomes_absolute() {
        let cwd = std::env::current_dir().unwrap();
        let tmp = tempfile::NamedTempFile::new_in(&cwd).unwrap();
        let relative = tmp.path().file_name().unwrap();
        let (display, _) = normalize(relative).unwrap();
        assert!(Path::new(&display).is_absolute());
        assert!(!display.starts_with(r"\\?\"));
    }

    #[test]
    fn missing_path_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.html");
        let err = normalize(&missing).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
