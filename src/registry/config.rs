use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::registry::Registry;

/// A single plain-config file or directory to back up and restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRegistryEntry {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    /// Path relative to a backup layer directory (e.g. `.bashrc`).
    pub backup_path: String,
    /// Absolute path on disk this entry is backed up from / restored to.
    pub original_path: PathBuf,
}

crate::impl_registry_entry_like!(ConfigRegistryEntry);

/// Registry of plain config files/directories, stored at
/// `config.registry.json`.
pub type ConfigRegistry = Registry<ConfigRegistryEntry>;

impl Default for ConfigRegistry {
    /// Built-in entries for common shell configs (`.bashrc`, `.zshrc`).
    fn default() -> Self {
        let mut entries = HashMap::new();

        let home_dir = dirs::home_dir().expect(
            "failed to get user home directory: $HOME not set or platform dirs unavailable",
        );

        entries.insert(
            "bashrc".to_string(),
            ConfigRegistryEntry {
                name: "Bash Configuration".to_string(),
                backup_path: ".bashrc".to_string(),
                original_path: home_dir.join(".bashrc"),
                enabled: true,
                description: Some("Bash shell configuration file".to_string()),
            },
        );

        entries.insert(
            "zshrc".to_string(),
            ConfigRegistryEntry {
                name: "Zsh Configuration".to_string(),
                backup_path: ".zshrc".to_string(),
                original_path: home_dir.join(".zshrc"),
                enabled: true,
                description: Some("Zsh shell configuration file".to_string()),
            },
        );

        Self {
            version: "1.0.0".to_string(),
            entries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_expected_version() {
        let registry = ConfigRegistry::default();
        assert_eq!(registry.version, "1.0.0");
    }

    #[test]
    fn default_includes_bashrc_entry() {
        let registry = ConfigRegistry::default();
        let bashrc = registry.entries.get("bashrc").unwrap();

        assert_eq!(bashrc.name, "Bash Configuration");
        assert_eq!(bashrc.backup_path, ".bashrc");
        assert!(bashrc.enabled);
        assert_eq!(
            bashrc.description.as_deref(),
            Some("Bash shell configuration file")
        );
        assert!(bashrc.original_path.ends_with(".bashrc"));
    }

    #[test]
    fn default_includes_zshrc_entry() {
        let registry = ConfigRegistry::default();
        let zshrc = registry.entries.get("zshrc").unwrap();

        assert_eq!(zshrc.name, "Zsh Configuration");
        assert_eq!(zshrc.backup_path, ".zshrc");
        assert!(zshrc.enabled);
        assert_eq!(
            zshrc.description.as_deref(),
            Some("Zsh shell configuration file")
        );
        assert!(zshrc.original_path.ends_with(".zshrc"));
    }

    #[test]
    fn default_has_exactly_two_entries() {
        let registry = ConfigRegistry::default();
        assert_eq!(registry.entries.len(), 2);
    }
}
