//! JSON-backed registries describing what dfm backs up: plain configs,
//! package-manager exports, and encrypted files.

pub mod config;
pub mod encrypted;
pub mod package;

use std::path::Path;
use std::{collections::BTreeMap, collections::HashMap};

pub use config::{ConfigRegistry, ConfigRegistryEntry};
pub use encrypted::{EncryptedRegistry, EncryptedRegistryEntry};
pub use package::{PackageRegistry, PackageRegistryEntry};
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Implemented by registry entry types so [`Registry`] can filter on the
/// shared `enabled` flag.
pub trait RegistryEntryLike {
    /// Whether this entry should be processed during backup/restore.
    fn is_enabled(&self) -> bool;
}

/// Implements [`RegistryEntryLike`] for a type with an `enabled: bool` field.
#[macro_export]
macro_rules! impl_registry_entry_like {
    ($t:ty) => {
        impl $crate::registry::RegistryEntryLike for $t {
            fn is_enabled(&self) -> bool {
                self.enabled
            }
        }
    };
}

/// A JSON-backed map of registry entries stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry<T> {
    pub version: String,
    pub entries: HashMap<String, T>,
}

impl<T> Registry<T>
where
    T: RegistryEntryLike + Clone + Serialize + for<'a> Deserialize<'a>,
{
    /// Load the registry at `path`, creating it with default contents when
    /// the file does not exist yet.
    pub fn load_or_create(path: &Path) -> Result<Self>
    where
        Self: Default,
    {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let registry: Registry<T> = serde_json::from_str(&content)?;
            Ok(registry)
        } else {
            let registry = Self::default();
            registry.save(path)?;
            Ok(registry)
        }
    }

    /// Write the registry to `path` as pretty-printed JSON, sorted by entry
    /// id for a stable diff.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let sorted_entries: BTreeMap<&String, &T> = self.entries.iter().collect();
        let sorted_registry = serde_json::json!({
            "version": self.version,
            "entries": sorted_entries
        });

        let content = serde_json::to_string_pretty(&sorted_registry)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Entries with `enabled: true`, keyed by id.
    pub fn get_enabled_entries(&self) -> impl Iterator<Item = (&String, &T)> {
        self.entries.iter().filter(|(_, e)| e.is_enabled())
    }

    /// All entries, keyed by id, when `include_disabled` is `true`;
    /// otherwise the same as [`Self::get_enabled_entries`].
    pub fn get_entries(&self, include_disabled: bool) -> impl Iterator<Item = (&String, &T)> {
        self.entries
            .iter()
            .filter(move |(_, e)| include_disabled || e.is_enabled())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::config::{ConfigRegistry, ConfigRegistryEntry};

    fn entry(name: &str, enabled: bool) -> ConfigRegistryEntry {
        ConfigRegistryEntry {
            name: name.to_string(),
            description: None,
            enabled,
            backup_path: format!(".{name}"),
            original_path: std::path::PathBuf::from(format!("/home/user/.{name}")),
        }
    }

    #[test]
    fn load_or_create_creates_default_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.registry.json");

        assert!(!path.exists());
        let registry = ConfigRegistry::load_or_create(&path).unwrap();
        assert!(path.exists());
        assert_eq!(
            registry.entries.len(),
            ConfigRegistry::default().entries.len()
        );
        assert!(registry.entries.contains_key("bashrc"));
        assert!(registry.entries.contains_key("zshrc"));
    }

    #[test]
    fn load_or_create_parses_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.registry.json");

        let mut registry: Registry<ConfigRegistryEntry> = Registry {
            version: "9.9.9".to_string(),
            entries: HashMap::new(),
        };
        registry
            .entries
            .insert("custom".to_string(), entry("custom", true));
        registry.save(&path).unwrap();

        let loaded = ConfigRegistry::load_or_create(&path).unwrap();
        assert_eq!(loaded.version, "9.9.9");
        assert_eq!(loaded.entries.len(), 1);
        assert!(loaded.entries.contains_key("custom"));
    }

    #[test]
    fn save_writes_pretty_printed_json_sorted_by_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.registry.json");

        let mut registry: Registry<ConfigRegistryEntry> = Registry {
            version: "1.0.0".to_string(),
            entries: HashMap::new(),
        };
        registry
            .entries
            .insert("zshrc".to_string(), entry("zshrc", true));
        registry
            .entries
            .insert("bashrc".to_string(), entry("bashrc", true));
        registry
            .entries
            .insert("aliases".to_string(), entry("aliases", true));
        registry.save(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\n"));

        let aliases_pos = content.find("\"aliases\"").unwrap();
        let bashrc_pos = content.find("\"bashrc\"").unwrap();
        let zshrc_pos = content.find("\"zshrc\"").unwrap();
        assert!(aliases_pos < bashrc_pos);
        assert!(bashrc_pos < zshrc_pos);
    }

    #[test]
    fn get_enabled_entries_filters_disabled() {
        let mut registry: Registry<ConfigRegistryEntry> = Registry {
            version: "1.0.0".to_string(),
            entries: HashMap::new(),
        };
        registry.entries.insert("on".to_string(), entry("on", true));
        registry
            .entries
            .insert("off".to_string(), entry("off", false));

        let enabled: Vec<&String> = registry.get_enabled_entries().map(|(id, _)| id).collect();
        assert_eq!(enabled, vec!["on"]);
    }
}
