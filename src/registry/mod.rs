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
    fn is_enabled(&self) -> bool;
}

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

    pub fn get_enabled_entries(&self) -> impl Iterator<Item = (&String, &T)> {
        self.entries.iter().filter(|(_, e)| e.is_enabled())
    }
}
