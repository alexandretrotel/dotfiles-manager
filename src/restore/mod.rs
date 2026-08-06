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
    pub profile: ActiveProfile,
    pub configs: SectionReport,
    /// `None` when encrypted restore was skipped (no password supplied).
    pub encrypted: Option<SectionReport>,
}

impl RestoreReport {
    /// Total number of configs (plain and encrypted) restored.
    pub fn restored(&self) -> usize {
        self.configs.succeeded() + self.encrypted.as_ref().map_or(0, |s| s.succeeded())
    }

    /// Total number of configs (plain and encrypted) skipped.
    pub fn skipped(&self) -> usize {
        self.configs.skipped() + self.encrypted.as_ref().map_or(0, |s| s.skipped())
    }
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
        let outcome = match profile.resolve_source(ctx, &entry.source_path) {
            Some(resolved) => match config::restore_config(&resolved.path, &entry.target_path) {
                Ok(()) => RegistryEntryOutcome::done(id, &entry.source_path),
                Err(reason) => RegistryEntryOutcome::skipped(id, &entry.source_path, reason),
            },
            None => RegistryEntryOutcome::skipped(id, &entry.source_path, "no backup in any layer"),
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
    use crate::report::RegistryEntryStatus;
    use std::collections::HashMap;
    use std::fs;

    fn section_with(outcomes: Vec<RegistryEntryOutcome>) -> SectionReport {
        SectionReport {
            outcomes,
            warnings: Vec::new(),
        }
    }

    fn done(id: &str) -> RegistryEntryOutcome {
        RegistryEntryOutcome {
            id: id.to_string(),
            label: id.to_string(),
            status: RegistryEntryStatus::Done { note: None },
        }
    }

    fn skipped(id: &str) -> RegistryEntryOutcome {
        RegistryEntryOutcome {
            id: id.to_string(),
            label: id.to_string(),
            status: RegistryEntryStatus::Skipped {
                reason: "test".to_string(),
            },
        }
    }

    #[test]
    fn restored_and_skipped_count_plain_configs_only_when_encrypted_is_none() {
        let report = RestoreReport {
            profile: ActiveProfile::common_only(),
            configs: section_with(vec![done("a"), done("b"), skipped("c")]),
            encrypted: None,
        };

        assert_eq!(report.restored(), 2);
        assert_eq!(report.skipped(), 1);
    }

    #[test]
    fn restored_and_skipped_sum_plain_and_encrypted_sections() {
        let report = RestoreReport {
            profile: ActiveProfile::common_only(),
            configs: section_with(vec![done("a"), skipped("b")]),
            encrypted: Some(section_with(vec![done("c"), done("d"), skipped("e")])),
        };

        assert_eq!(report.restored(), 3);
        assert_eq!(report.skipped(), 2);
    }

    #[test]
    fn restore_run_restores_a_config_from_the_common_layer() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();

        let backup_file = profile.backup_path(&ctx).join("myfile.txt");
        fs::create_dir_all(backup_file.parent().unwrap()).unwrap();
        fs::write(&backup_file, b"backed up content").unwrap();

        let target = dir.path().join("restored/myfile.txt");

        let mut entries = HashMap::new();
        entries.insert(
            "myfile".to_string(),
            ConfigRegistryEntry {
                name: "My File".to_string(),
                description: None,
                enabled: true,
                source_path: "myfile.txt".to_string(),
                target_path: target.clone(),
            },
        );
        let registry = ConfigRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.config_registry_path()).unwrap();

        let report = run(&ctx, &profile, None).unwrap();

        assert_eq!(report.restored(), 1);
        assert!(report.encrypted.is_none());
        assert_eq!(fs::read(&target).unwrap(), b"backed up content");
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
                source_path: "ghost.txt".to_string(),
                target_path: dir.path().join("ghost-target.txt"),
            },
        );
        let registry = ConfigRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.config_registry_path()).unwrap();

        let report = run(&ctx, &profile, None).unwrap();

        assert_eq!(report.restored(), 0);
        assert_eq!(report.skipped(), 1);
        assert!(!dir.path().join("ghost-target.txt").exists());
    }
}
