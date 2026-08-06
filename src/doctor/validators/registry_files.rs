use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;

use super::Validator;
use crate::context::Dfm;
use crate::doctor::report::ValidationError;
use crate::registry::{ConfigRegistry, PackageRegistry};
use crate::utils::process::is_command_available;

/// Checks the config and package registries parse, have no duplicate source
/// paths, and reference package managers available on `PATH`.
pub(super) struct RegistryFilesValidator {
    ctx: Dfm,
}

impl RegistryFilesValidator {
    /// Build the validator for `ctx`'s registry files.
    pub(super) fn new(ctx: Dfm) -> Self {
        Self { ctx }
    }
}

/// Read and parse a registry file, pushing a warning/error/info finding on
/// failure. `label` is the capitalized display name (e.g. "Config").
fn load_registry<T: serde::de::DeserializeOwned>(
    path: &Path,
    label: &str,
    errors: &mut Vec<ValidationError>,
) -> Option<T> {
    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<T>(&content) {
            Ok(registry) => Some(registry),
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not parse {} registry: {}",
                    label.to_lowercase(),
                    e
                )));
                None
            }
        },
        Err(e) if e.kind() == ErrorKind::NotFound => {
            errors.push(ValidationError::info(format!(
                "{} registry file not found",
                label
            )));
            None
        }
        Err(e) => {
            errors.push(ValidationError::error(format!(
                "Could not read {} registry: {}",
                label.to_lowercase(),
                e
            )));
            None
        }
    }
}

impl Validator for RegistryFilesValidator {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        let config_registry_path = self.ctx.config_registry_path();
        if let Some(registry) =
            load_registry::<ConfigRegistry>(&config_registry_path, "Config", &mut errors)
        {
            let mut source_paths: HashMap<String, Vec<String>> = HashMap::new();
            for (id, entry) in registry.entries.iter() {
                source_paths
                    .entry(entry.source_path.clone())
                    .or_default()
                    .push(id.clone());
            }
            for (path, ids) in source_paths {
                if ids.len() > 1 {
                    errors.push(
                        ValidationError::warning(format!(
                            "Duplicate source path '{}' used by: {}",
                            path,
                            ids.join(", ")
                        ))
                        .with_fix("Consider consolidating or renaming entries"),
                    );
                }
            }
        }

        let package_registry_path = self.ctx.package_registry_path();
        if let Some(registry) =
            load_registry::<PackageRegistry>(&package_registry_path, "Package", &mut errors)
        {
            for (id, entry) in registry.get_platform_compatible_entries(std::env::consts::OS) {
                if !is_command_available(&entry.command) {
                    errors.push(
                        ValidationError::info(format!(
                            "Package manager '{}' ({}) not found in PATH",
                            entry.name, id
                        ))
                        .with_fix(format!(
                            "Install {} or disable this entry in your profile config",
                            entry.command
                        )),
                    );
                }
            }
        }

        errors
    }

    fn name(&self) -> &str {
        "Registry Files"
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::context::Dfm;
    use crate::registry::{ConfigRegistryEntry, PackageRegistryEntry};

    fn config_entry(source_path: &str) -> ConfigRegistryEntry {
        ConfigRegistryEntry {
            name: "Test Entry".to_string(),
            description: None,
            enabled: true,
            source_path: source_path.to_string(),
            target_path: std::path::PathBuf::from("/tmp/does-not-matter"),
        }
    }

    #[test]
    fn name_is_registry_files() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        assert_eq!(RegistryFilesValidator::new(ctx).name(), "Registry Files");
    }

    #[test]
    fn missing_registry_files_produce_info_findings() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        let validator = RegistryFilesValidator::new(ctx);

        let errors = validator.validate();

        assert!(
            errors
                .iter()
                .any(|e| e.severity == crate::doctor::report::Severity::Info
                    && e.message == "Config registry file not found")
        );
        assert!(
            errors
                .iter()
                .any(|e| e.severity == crate::doctor::report::Severity::Info
                    && e.message == "Package registry file not found")
        );
    }

    #[test]
    fn invalid_config_json_produces_error_finding() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        std::fs::write(ctx.config_registry_path(), "not valid json {").unwrap();
        let validator = RegistryFilesValidator::new(ctx);

        let errors = validator.validate();

        assert!(errors.iter().any(|e| {
            e.severity == crate::doctor::report::Severity::Error
                && e.message.contains("Could not parse config registry")
        }));
    }

    #[test]
    fn valid_config_registry_with_unique_source_paths_produces_no_errors() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let mut entries = HashMap::new();
        entries.insert("bashrc".to_string(), config_entry(".bashrc"));
        entries.insert("zshrc".to_string(), config_entry(".zshrc"));
        let registry = ConfigRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.config_registry_path()).unwrap();

        let validator = RegistryFilesValidator::new(ctx);
        let errors = validator.validate();

        assert!(
            !errors
                .iter()
                .any(|e| e.severity == crate::doctor::report::Severity::Error)
        );
    }

    #[test]
    fn duplicate_source_path_produces_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let mut entries = HashMap::new();
        entries.insert("bashrc_a".to_string(), config_entry(".bashrc"));
        entries.insert("bashrc_b".to_string(), config_entry(".bashrc"));
        let registry = ConfigRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.config_registry_path()).unwrap();

        let validator = RegistryFilesValidator::new(ctx);
        let errors = validator.validate();

        assert!(errors.iter().any(|e| {
            e.severity == crate::doctor::report::Severity::Warning
                && e.message.contains("Duplicate source path '.bashrc'")
        }));
    }

    #[test]
    fn package_entry_with_unavailable_command_produces_info_finding() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let mut entries = HashMap::new();
        entries.insert(
            "fake_pm".to_string(),
            PackageRegistryEntry {
                name: "Fake Package Manager".to_string(),
                description: None,
                enabled: true,
                command: "definitely-not-a-real-pkg-mgr-xyz".to_string(),
                args: vec![],
                output_file: "fake.txt".to_string(),
                platforms: None,
            },
        );
        let registry = PackageRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.package_registry_path()).unwrap();

        let validator = RegistryFilesValidator::new(ctx);
        let errors = validator.validate();

        assert!(errors.iter().any(|e| {
            e.severity == crate::doctor::report::Severity::Info
                && e.message.contains("Fake Package Manager")
                && e.message.contains("not found in PATH")
        }));
    }
}
