use std::path::Path;

use super::model::{Binding, Settings};

const SKIPPED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "$recycle.bin",
    "system volume information",
];

const TEMP_EXTENSIONS: &[&str] = &[".tmp", ".crdownload", ".part"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub extensions: Vec<String>,
}

pub fn effective_rule(binding: &Binding, settings: &Settings) -> Rule {
    let extensions = if binding.extensions.is_empty() {
        settings.extensions.clone()
    } else {
        binding.extensions.clone()
    };
    Rule { extensions }
}

pub fn matches(path: impl AsRef<Path>, extensions: &[String]) -> bool {
    let Some(ext) = extension(path.as_ref()) else {
        return false;
    };
    extensions
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&ext))
}

pub fn is_temp_file(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    if file_name(path).is_some_and(|name| name.starts_with("~$")) {
        return true;
    }
    extension(path).is_some_and(|ext| {
        TEMP_EXTENSIONS
            .iter()
            .any(|temp| ext.eq_ignore_ascii_case(temp))
    })
}

pub fn is_skipped_dir(name: &str) -> bool {
    SKIPPED_DIRS
        .iter()
        .any(|skipped| name.eq_ignore_ascii_case(skipped))
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}

fn extension(path: &Path) -> Option<String> {
    path.extension()?
        .to_str()
        .map(|ext| format!(".{}", ext.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{Binding, Settings};
    use chrono::TimeZone;

    fn dummy_binding() -> Binding {
        Binding {
            id: "b1".to_string(),
            path: r"D:\html".to_string(),
            key: r"d:\html".to_string(),
            recursive: false,
            auto_watch: false,
            extensions: Vec::new(),
            rule_id: Some("custom".to_string()),
            created_at: chrono::Local
                .with_ymd_and_hms(2026, 11, 4, 15, 0, 0)
                .unwrap(),
            unavailable: false,
        }
    }

    fn default_rule() -> Rule {
        effective_rule(&dummy_binding(), &Settings::default())
    }

    #[test]
    fn html_matches_default_rule_case_insensitively() {
        let rule = default_rule();
        assert!(matches("page.HTML", &rule.extensions));
        assert!(!matches("page.htm", &rule.extensions));
    }

    #[test]
    fn binding_extensions_override_settings() {
        let mut binding = dummy_binding();
        binding.extensions = vec![".md".to_string()];
        let rule = effective_rule(
            &binding,
            &Settings {
                extensions: vec![".html".to_string()],
            },
        );
        assert!(matches("note.md", &rule.extensions));
        assert!(!matches("page.html", &rule.extensions));
    }

    #[test]
    fn html_bak_does_not_match() {
        let rule = default_rule();
        assert!(!matches("page.html.bak", &rule.extensions));
    }

    #[test]
    fn office_lock_and_download_suffixes_are_temp_files() {
        assert!(is_temp_file("~$a.html"));
        assert!(is_temp_file("a.html.crdownload"));
        assert!(is_temp_file("a.tmp"));
        assert!(!is_temp_file("a.html"));
    }

    #[test]
    fn node_modules_and_git_are_skipped_case_insensitively() {
        assert!(is_skipped_dir("node_modules"));
        assert!(is_skipped_dir("NODE_MODULES"));
        assert!(is_skipped_dir(".git"));
        assert!(is_skipped_dir(".GIT"));
        assert!(!is_skipped_dir("src"));
    }
}
