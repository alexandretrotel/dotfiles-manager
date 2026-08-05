use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::registry::{Registry, RegistryEntryLike};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRegistryEntry {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    pub output_file: String,
    pub platforms: Option<Vec<String>>,
}

impl RegistryEntryLike for PackageRegistryEntry {
    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

pub type PackageRegistry = Registry<PackageRegistryEntry>;

impl Default for PackageRegistry {
    fn default() -> Self {
        let mut entries = HashMap::new();

        entries.insert(
            "brew".to_string(),
            PackageRegistryEntry {
                name: "Homebrew".to_string(),
                command: "brew".to_string(),
                args: vec!["leaves".to_string()],
                output_file: "brew.txt".to_string(),
                enabled: true,
                description: Some("Homebrew installed packages".to_string()),
                platforms: Some(vec!["macos".to_string(), "linux".to_string()]),
            },
        );

        entries.insert(
            "npm".to_string(),
            PackageRegistryEntry {
                name: "npm".to_string(),
                command: "npm".to_string(),
                args: vec!["ls".to_string(), "-g".to_string()],
                output_file: "npm.txt".to_string(),
                enabled: true,
                description: Some("npm globally installed packages".to_string()),
                platforms: None,
            },
        );

        entries.insert(
            "cargo".to_string(),
            PackageRegistryEntry {
                name: "Cargo".to_string(),
                command: "cargo".to_string(),
                args: vec!["install".to_string(), "--list".to_string()],
                output_file: "cargo.txt".to_string(),
                enabled: true,
                description: Some("Cargo installed packages".to_string()),
                platforms: None,
            },
        );

        entries.insert(
            "uv".to_string(),
            PackageRegistryEntry {
                name: "uv".to_string(),
                command: "uv".to_string(),
                args: vec!["tool".to_string(), "list".to_string()],
                output_file: "uv.txt".to_string(),
                enabled: true,
                description: Some("uv installed tools".to_string()),
                platforms: None,
            },
        );

        Self {
            version: "1.0.0".to_string(),
            entries,
        }
    }
}

impl PackageRegistry {
    pub fn get_platform_compatible_entries<'a>(
        &'a self,
        current_platform: &'a str,
    ) -> impl Iterator<Item = (&'a String, &'a PackageRegistryEntry)> + 'a {
        self.entries.iter().filter(move |(_, entry)| {
            entry.enabled
                && match &entry.platforms {
                    Some(platforms) => platforms.contains(&current_platform.to_string()),
                    None => true,
                }
        })
    }

    pub fn get_current_platform() -> String {
        #[cfg(target_os = "macos")]
        return "macos".into();
        #[cfg(target_os = "linux")]
        return "linux".into();
        #[cfg(target_os = "windows")]
        return "windows".into();
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        return "unknown".into();
    }
}
