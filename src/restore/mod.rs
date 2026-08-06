mod config;
mod encrypted;

use age::secrecy::SecretString;

use crate::context::Dfm;
use crate::error::{Result, WrapErr};
use crate::profiles::ActiveProfile;
use crate::registry::ConfigRegistry;
use crate::report::{ItemOutcome, SectionReport};

/// Everything a restore run produced.
#[derive(Debug, Clone)]
pub struct RestoreReport {
    pub profile: ActiveProfile,
    pub configs: SectionReport,
    /// `None` when encrypted restore was skipped (no password supplied).
    pub encrypted: Option<SectionReport>,
}

impl RestoreReport {
    pub fn restored(&self) -> usize {
        self.configs.succeeded() + self.encrypted.as_ref().map_or(0, |s| s.succeeded())
    }

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
                Ok(()) => ItemOutcome::done(id, &entry.source_path),
                Err(reason) => ItemOutcome::skipped(id, &entry.source_path, reason),
            },
            None => ItemOutcome::skipped(id, &entry.source_path, "no backup in any layer"),
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
