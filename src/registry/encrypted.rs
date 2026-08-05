use std::{collections::HashMap, path::PathBuf};

use directories_next::BaseDirs;
use serde::{Deserialize, Serialize};

use crate::registry::Registry;

/// A single sensitive file, backed up age-encrypted rather than plaintext.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedRegistryEntry {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    /// Path relative to a backup layer's encrypted directory, and the
    /// archive member name inside the encrypted bundle.
    pub source_path: String,
    /// Absolute path on disk this entry is backed up from / restored to.
    pub target_path: PathBuf,
}

crate::impl_registry_entry_like!(EncryptedRegistryEntry);

/// Registry of sensitive files to back up encrypted, stored at
/// `encrypted.registry.json`.
pub type EncryptedRegistry = Registry<EncryptedRegistryEntry>;

impl Default for EncryptedRegistry {
    /// Built-in entry for `~/.ssh/config`.
    fn default() -> Self {
        let mut entries = HashMap::new();

        let base_dirs = BaseDirs::new()
            .expect("failed to get user base dirs: $HOME not set or platform dirs unavailable");
        let home_dir = base_dirs.home_dir();

        entries.insert(
            "ssh_config".to_string(),
            EncryptedRegistryEntry {
                name: "SSH Config".to_string(),
                source_path: "ssh/config".to_string(),
                target_path: home_dir.join(".ssh/config"),
                enabled: true,
                description: Some("SSH client configuration file".to_string()),
            },
        );

        Self {
            version: "1.0.0".to_string(),
            entries,
        }
    }
}
