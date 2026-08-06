//! Back up configs, package-manager lists, and encrypted configs to the
//! dfm root.

mod config;
mod encrypted;
mod fs_ops;
mod package;

use std::fs;

use age::secrecy::SecretString;

use crate::context::Dfm;
use crate::error::Result;
use crate::git;
use crate::profiles::ActiveProfile;
use crate::report::SectionReport;

/// Everything a backup run produced.
#[derive(Debug, Clone)]
pub struct BackupReport {
    pub profile: ActiveProfile,
    pub repo: git::InitReport,
    pub configs: SectionReport,
    pub packages: SectionReport,
    /// `None` when encrypted backup was skipped (no password supplied).
    pub encrypted: Option<SectionReport>,
}

/// Back up configs, package lists, and (when `password` is given) encrypted
/// configs for `profile`.
pub fn run(
    ctx: &Dfm,
    profile: &ActiveProfile,
    password: Option<&SecretString>,
) -> Result<BackupReport> {
    let repo = git::init_repo_if_missing(ctx)?;

    let backup_path = profile.backup_path(ctx);
    fs::create_dir_all(&backup_path)?;

    let packages_path = ctx.packages_dir();
    fs::create_dir_all(&packages_path)?;

    let configs = config::backup_configs(ctx, &backup_path)?;
    let packages = package::backup_packages(ctx, &packages_path)?;

    let encrypted = match password {
        Some(password) => {
            let encrypted_backup_path = profile.encrypted_backup_path(ctx);
            fs::create_dir_all(&encrypted_backup_path)?;
            Some(encrypted::backup_encrypted_configs(
                ctx,
                &encrypted_backup_path,
                password,
            )?)
        }
        None => None,
    };

    Ok(BackupReport {
        profile: profile.clone(),
        repo,
        configs,
        packages,
        encrypted,
    })
}
