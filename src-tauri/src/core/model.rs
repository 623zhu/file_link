use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub version: u32,
    pub settings: Settings,
    pub bindings: Vec<Binding>,
    pub links: Vec<Link>,
    pub ignored: Vec<Ignore>,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            version: 2,
            settings: Settings::default(),
            bindings: Vec::new(),
            links: Vec::new(),
            ignored: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub extensions: Vec<String>,
}

impl Settings {
    pub fn html_extensions() -> Vec<String> {
        vec![".html".to_string()]
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            extensions: Self::html_extensions(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    pub id: String,
    pub path: String,
    pub key: String,
    pub recursive: bool,
    pub auto_watch: bool,
    #[serde(default)]
    pub extensions: Vec<String>,
    pub rule_id: Option<String>,
    pub created_at: DateTime<Local>,
    #[serde(default)]
    pub unavailable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub id: String,
    pub path: String,
    pub key: String,
    pub name: String,
    pub sources: Vec<Source>,
    pub added_at: DateTime<Local>,
    #[serde(default)]
    pub missing: bool,
}

impl Link {
    pub fn has_source(&self, source: &Source) -> bool {
        self.sources.contains(source)
    }

    pub fn add_source(&mut self, source: Source) {
        if !self.has_source(&source) {
            self.sources.push(source);
        }
    }

    pub fn remove_binding_source(&mut self, binding_id: &str) {
        self.sources.retain(|source| match source {
            Source::Binding { binding_id: id } => id != binding_id,
            Source::Manual => true,
        });
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Source {
    Manual,
    #[serde(rename_all = "camelCase")]
    Binding {
        binding_id: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Ignore {
    pub binding_id: String,
    pub key: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::{Value, json};

    fn sample_time() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 4, 15, 0, 0).unwrap()
    }

    fn sample_binding() -> Binding {
        Binding {
            id: "b1".to_string(),
            path: r"D:\tools".to_string(),
            key: r"d:\tools".to_string(),
            recursive: true,
            auto_watch: true,
            extensions: Settings::html_extensions(),
            rule_id: None,
            created_at: sample_time(),
            unavailable: false,
        }
    }

    fn sample_link() -> Link {
        Link {
            id: "l1".to_string(),
            path: r"D:\tools\json.html".to_string(),
            key: r"d:\tools\json.html".to_string(),
            name: "json.html".to_string(),
            sources: vec![Source::Manual],
            added_at: sample_time(),
            missing: false,
        }
    }

    fn sample_data() -> Data {
        Data {
            version: 2,
            settings: Settings::default(),
            bindings: vec![sample_binding()],
            links: vec![sample_link()],
            ignored: vec![Ignore {
                binding_id: "b1".to_string(),
                key: r"d:\tools\old.html".to_string(),
            }],
        }
    }

    fn assert_has_keys(value: &Value, keys: &[&str]) {
        let object = value.as_object().expect("expected JSON object");
        for key in keys {
            assert!(object.contains_key(*key), "missing {key}");
        }
        assert_eq!(object.len(), keys.len());
    }

    #[test]
    fn data_default_is_version_two_with_html_extensions() {
        let data = Data::default();
        assert_eq!(data.version, 2);
        assert_eq!(data.settings.extensions, [".html"]);
        assert!(data.bindings.is_empty());
        assert!(data.links.is_empty());
        assert!(data.ignored.is_empty());
    }

    #[test]
    fn data_serializes_camel_case_fields() {
        let value = serde_json::to_value(sample_data()).unwrap();
        assert_has_keys(
            &value,
            &["version", "settings", "bindings", "links", "ignored"],
        );

        let binding = &value["bindings"][0];
        for key in [
            "autoWatch",
            "extensions",
            "ruleId",
            "createdAt",
            "unavailable",
            "recursive",
        ] {
            assert!(binding.get(key).is_some(), "missing {key}");
        }
        assert!(binding.get("auto_watch").is_none());
        assert!(binding.get("created_at").is_none());

        let link = &value["links"][0];
        assert!(link.get("addedAt").is_some());
        assert!(link.get("added_at").is_none());

        let ignored = &value["ignored"][0];
        assert!(ignored.get("bindingId").is_some());
        assert!(ignored.get("binding_id").is_none());
    }

    #[test]
    fn source_serializes_as_tagged_camel_case() {
        assert_eq!(
            serde_json::to_value(Source::Manual).unwrap(),
            json!({ "type": "manual" })
        );
        assert_eq!(
            serde_json::to_value(Source::Binding {
                binding_id: "b1".to_string(),
            })
            .unwrap(),
            json!({ "type": "binding", "bindingId": "b1" })
        );
    }

    #[test]
    fn data_roundtrips_through_json() {
        let data = sample_data();
        let json = serde_json::to_string(&data).unwrap();
        let restored: Data = serde_json::from_str(&json).unwrap();
        assert_eq!(data, restored);
    }

    #[test]
    fn missing_and_unavailable_default_when_absent() {
        let mut binding = serde_json::to_value(sample_binding()).unwrap();
        binding.as_object_mut().unwrap().remove("unavailable");
        binding.as_object_mut().unwrap().remove("extensions");
        let binding: Binding = serde_json::from_value(binding).unwrap();
        assert!(!binding.unavailable);
        assert!(binding.extensions.is_empty());

        let mut link = serde_json::to_value(sample_link()).unwrap();
        link.as_object_mut().unwrap().remove("missing");
        let link: Link = serde_json::from_value(link).unwrap();
        assert!(!link.missing);
    }

    #[test]
    fn add_source_does_not_duplicate() {
        let mut link = sample_link();
        link.add_source(Source::Manual);
        link.add_source(Source::Binding {
            binding_id: "b1".to_string(),
        });
        link.add_source(Source::Binding {
            binding_id: "b1".to_string(),
        });

        assert_eq!(
            link.sources,
            [
                Source::Manual,
                Source::Binding {
                    binding_id: "b1".to_string(),
                }
            ]
        );
        assert!(link.has_source(&Source::Manual));
        assert!(link.has_source(&Source::Binding {
            binding_id: "b1".to_string(),
        }));
    }

    #[test]
    fn remove_binding_source_keeps_other_sources() {
        let mut link = sample_link();
        link.add_source(Source::Binding {
            binding_id: "b1".to_string(),
        });
        link.add_source(Source::Binding {
            binding_id: "b2".to_string(),
        });

        link.remove_binding_source("b1");

        assert_eq!(
            link.sources,
            [
                Source::Manual,
                Source::Binding {
                    binding_id: "b2".to_string(),
                }
            ]
        );
        assert!(!link.has_source(&Source::Binding {
            binding_id: "b1".to_string(),
        }));
    }
}
