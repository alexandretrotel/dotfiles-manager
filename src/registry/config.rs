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
    pub source_path: String,
    /// Absolute path on disk this entry is backed up from / restored to.
    pub target_path: PathBuf,
}

crate::impl_registry_entry_like!(ConfigRegistryEntry);

/// Registry of plain config files/directories, stored at
/// `config.registry.json`.
pub type ConfigRegistry = Registry<ConfigRegistryEntry>;

impl Default for ConfigRegistry {
    /// Built-in entries for common shell configs (`.bashrc`, `.zshrc`).
    fn default() -> Self {
        let mut entries = HashMap::new();

        let home_dir = dirs::home_dir()
            .expect("failed to get user home directory: $HOME not set or platform dirs unavailable");

        entries.insert(
            "bashrc".to_string(),
            ConfigRegistryEntry {
                name: "Bash Configuration".to_string(),
                source_path: ".bashrc".to_string(),
                target_path: home_dir.join(".bashrc"),
                enabled: true,
                description: Some("Bash shell configuration file".to_string()),
            },
        );

        entries.insert(
            "zshrc".to_string(),
            ConfigRegistryEntry {
                name: "Zsh Configuration".to_string(),
                source_path: ".zshrc".to_string(),
                target_path: home_dir.join(".zshrc"),
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
