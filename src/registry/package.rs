use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::registry::Registry;

/// A package manager whose installed-package list is exported to a file
/// during backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRegistryEntry {
    /// Human-readable label for this entry (e.g. `Homebrew`).
    pub name: String,
    /// Optional human-readable explanation of what this entry exports.
    pub description: Option<String>,
    /// Whether this entry is processed during backup/restore.
    pub enabled: bool,
    /// The package manager's executable name (e.g. `brew`).
    pub command: String,
    /// Arguments that make `command` print the installed-package list.
    pub args: Vec<String>,
    /// File name (under the packages directory) the export is written to.
    pub output_file: String,
    /// Platforms (`std::env::consts::OS` values) this entry applies to, or
    /// `None` for all platforms.
    pub platforms: Option<Vec<String>>,
}

crate::impl_registry_entry_like!(PackageRegistryEntry);

/// Registry of package managers to export, stored at
/// `package.registry.json`.
pub type PackageRegistry = Registry<PackageRegistryEntry>;

impl Default for PackageRegistry {
    /// Built-in entries for common package managers (Homebrew, npm, Cargo,
    /// uv).
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
    /// Enabled entries whose `platforms` include `current_platform` (or that
    /// apply to all platforms).
    pub fn get_platform_compatible_entries<'a>(
        &'a self,
        current_platform: &'a str,
    ) -> impl Iterator<Item = (&'a String, &'a PackageRegistryEntry)> + 'a {
        self.entries.iter().filter(move |(_, entry)| {
            entry.enabled
                && match &entry.platforms {
                    Some(platforms) => platforms.iter().any(|p| p == current_platform),
                    None => true,
                }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_expected_version() {
        let registry = PackageRegistry::default();
        assert_eq!(registry.version, "1.0.0");
    }

    #[test]
    fn default_includes_brew_entry_scoped_to_macos_and_linux() {
        let registry = PackageRegistry::default();
        let brew = registry.entries.get("brew").unwrap();

        assert_eq!(brew.name, "Homebrew");
        assert_eq!(brew.command, "brew");
        assert_eq!(brew.args, vec!["leaves".to_string()]);
        assert_eq!(brew.output_file, "brew.txt");
        assert!(brew.enabled);
        assert_eq!(
            brew.platforms,
            Some(vec!["macos".to_string(), "linux".to_string()])
        );
    }

    #[test]
    fn default_includes_npm_entry_with_no_platform_restriction() {
        let registry = PackageRegistry::default();
        let npm = registry.entries.get("npm").unwrap();

        assert_eq!(npm.name, "npm");
        assert_eq!(npm.command, "npm");
        assert!(npm.enabled);
        assert_eq!(npm.platforms, None);
    }

    #[test]
    fn default_has_four_entries() {
        let registry = PackageRegistry::default();
        assert_eq!(registry.entries.len(), 4);
    }

    #[test]
    fn platform_compatible_entries_includes_matching_platform() {
        let registry = PackageRegistry::default();
        let ids: Vec<&String> = registry
            .get_platform_compatible_entries("macos")
            .map(|(id, _)| id)
            .collect();

        assert!(ids.contains(&&"brew".to_string()));
    }

    #[test]
    fn platform_compatible_entries_excludes_non_matching_platform() {
        let registry = PackageRegistry::default();
        let ids: Vec<&String> = registry
            .get_platform_compatible_entries("windows")
            .map(|(id, _)| id)
            .collect();

        assert!(!ids.contains(&&"brew".to_string()));
    }

    #[test]
    fn platform_compatible_entries_always_includes_platform_agnostic_entries() {
        let registry = PackageRegistry::default();
        let ids: Vec<&String> = registry
            .get_platform_compatible_entries("windows")
            .map(|(id, _)| id)
            .collect();

        assert!(ids.contains(&&"npm".to_string()));
        assert!(ids.contains(&&"cargo".to_string()));
        assert!(ids.contains(&&"uv".to_string()));
    }

    #[test]
    fn platform_compatible_entries_excludes_disabled_entries() {
        let mut registry = PackageRegistry::default();
        registry.entries.get_mut("npm").unwrap().enabled = false;

        let ids: Vec<&String> = registry
            .get_platform_compatible_entries("linux")
            .map(|(id, _)| id)
            .collect();

        assert!(!ids.contains(&&"npm".to_string()));
    }
}
