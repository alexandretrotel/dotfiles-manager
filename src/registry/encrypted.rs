use std::{collections::HashMap, path::PathBuf};

use directories_next::BaseDirs;
use serde::{Deserialize, Serialize};

use crate::registry::{Registry, RegistryEntryLike};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedRegistryEntry {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub source_path: String,
    pub target_path: PathBuf,
}

impl RegistryEntryLike for EncryptedRegistryEntry {
    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

pub type EncryptedRegistry = Registry<EncryptedRegistryEntry>;

impl Default for EncryptedRegistry {
    fn default() -> Self {
        let mut entries = HashMap::new();

        let base_dirs = BaseDirs::new().unwrap();
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
