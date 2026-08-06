//! Restore backed-up configs and encrypted registry entries to their original locations.

mod config;
mod encrypted;

use age::secrecy::SecretString;

use crate::context::Dfm;
use crate::error::{Result, WrapErr};
use crate::profiles::ActiveProfile;
use crate::registry::ConfigRegistry;
use crate::report::{RegistryEntryOutcome, SectionReport};

/// Everything a restore run produced.
#[derive(Debug, Clone)]
pub struct RestoreReport {
    /// The profile the restore ran against.
    pub profile: ActiveProfile,
    /// Outcome of restoring plaintext config entries.
    pub configs: SectionReport,
    /// `None` when encrypted restore was skipped (no password supplied).
    pub encrypted: Option<SectionReport>,
}

/// Restore configs and (when `password` is given) encrypted configs for
/// `profile`.
pub fn run(
    ctx: &Dfm,
    profile: &ActiveProfile,
    password: Option<&SecretString>,
) -> Result<RestoreReport> {
    let config_registry_path = ctx.config_registry_path();
    let config_registry = ConfigRegistry::load_or_create(&config_registry_path)
        .wrap_err_with(|| format!("Load config registry: {}", config_registry_path.display()))?;

    let mut configs = SectionReport::default();

    for (id, entry) in config_registry.get_enabled_entries() {
        let outcome = match profile.resolve_backup_path(ctx, &entry.backup_path) {
            Some(resolved) => match config::restore_config(&resolved.path, &entry.original_path) {
                Ok(()) => RegistryEntryOutcome::done(id, &entry.backup_path),
                Err(reason) => RegistryEntryOutcome::skipped(id, &entry.backup_path, reason),
            },
            None => RegistryEntryOutcome::skipped(id, &entry.backup_path, "no backup in any layer"),
        };
        configs.outcomes.push(outcome);
    }

    let encrypted =
        password.map(|password| encrypted::restore_encrypted_configs(ctx, profile, password));

    Ok(RestoreReport {
        profile: profile.clone(),
        configs,
        encrypted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ConfigRegistryEntry;
    use std::collections::HashMap;
    use std::fs;

    #[test]
    fn restore_run_restores_a_config_from_the_common_layer() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();

        let backup_file = profile.backup_path(&ctx).join("myfile.txt");
        fs::create_dir_all(backup_file.parent().unwrap()).unwrap();
        fs::write(&backup_file, b"backed up content").unwrap();

        let original = dir.path().join("restored/myfile.txt");

        let mut entries = HashMap::new();
        entries.insert(
            "myfile".to_string(),
            ConfigRegistryEntry {
                name: "My File".to_string(),
                description: None,
                enabled: true,
                backup_path: "myfile.txt".to_string(),
                original_path: original.clone(),
            },
        );
        let registry = ConfigRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.config_registry_path()).unwrap();

        let report = run(&ctx, &profile, None).unwrap();

        assert_eq!(report.configs.succeeded(), 1);
        assert!(report.encrypted.is_none());
        assert_eq!(fs::read(&original).unwrap(), b"backed up content");
    }

    #[test]
    fn restore_run_skips_entries_with_no_backup_in_any_layer() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();

        let mut entries = HashMap::new();
        entries.insert(
            "ghost".to_string(),
            ConfigRegistryEntry {
                name: "Ghost".to_string(),
                description: None,
                enabled: true,
                backup_path: "ghost.txt".to_string(),
                original_path: dir.path().join("ghost-original.txt"),
            },
        );
        let registry = ConfigRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.config_registry_path()).unwrap();

        let report = run(&ctx, &profile, None).unwrap();

        assert_eq!(report.configs.succeeded(), 0);
        assert_eq!(report.configs.skipped(), 1);
        assert!(!dir.path().join("ghost-original.txt").exists());
    }
}
