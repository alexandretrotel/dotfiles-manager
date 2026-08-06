//! Back up configs, package-manager lists, and encrypted configs to the
//! dfm root.

mod config;
mod encrypted;
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
    /// The profile that was backed up.
    pub profile: ActiveProfile,
    /// Outcome of ensuring the dfm git repository exists.
    pub repo: git::InitReport,
    /// Outcome of backing up config registry entries.
    pub configs: SectionReport,
    /// Outcome of backing up package manager lists.
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

    let configs = config::backup_entries(ctx, &backup_path)?;
    let packages = package::backup_packages(ctx, &packages_path)?;

    let encrypted = match password {
        Some(password) => {
            let encrypted_backup_path = profile.encrypted_backup_path(ctx);
            fs::create_dir_all(&encrypted_backup_path)?;
            Some(encrypted::backup_encrypted_entries(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{
        ConfigRegistry, ConfigRegistryEntry, EncryptedRegistry, EncryptedRegistryEntry,
        PackageRegistry, PackageRegistryEntry,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn save_config_registry(ctx: &Dfm, backup_path: &str, original_path: PathBuf) {
        let mut entries = HashMap::new();
        entries.insert(
            "entry".to_string(),
            ConfigRegistryEntry {
                name: "Entry".to_string(),
                description: None,
                enabled: true,
                backup_path: backup_path.to_string(),
                original_path,
            },
        );
        let registry = ConfigRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.config_registry_path()).unwrap();
    }

    fn save_safe_package_registry(ctx: &Dfm) {
        let mut entries = HashMap::new();
        entries.insert(
            "fake".to_string(),
            PackageRegistryEntry {
                name: "Fake".to_string(),
                description: None,
                enabled: true,
                command: "echo".to_string(),
                args: vec!["installed-package".to_string()],
                output_file: "fake.txt".to_string(),
                platforms: None,
            },
        );
        let registry = PackageRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.package_registry_path()).unwrap();
    }

    #[test]
    fn run_backs_up_configs_and_packages_without_encryption() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();

        let original_file = dir.path().join("original.txt");
        fs::write(&original_file, b"config content").unwrap();
        save_config_registry(&ctx, "myfile.txt", original_file);
        save_safe_package_registry(&ctx);

        let report = run(&ctx, &profile, None).unwrap();

        assert!(report.repo.initialized);
        assert_eq!(report.configs.succeeded(), 1);
        assert_eq!(report.packages.succeeded(), 1);
        assert!(report.encrypted.is_none());

        let backup_file = profile.backup_path(&ctx).join("myfile.txt");
        assert_eq!(fs::read(&backup_file).unwrap(), b"config content");

        let package_file = ctx.packages_dir().join("fake.txt");
        assert_eq!(
            fs::read_to_string(&package_file).unwrap(),
            "installed-package\n"
        );
    }

    #[test]
    fn run_backs_up_encrypted_configs_when_password_given() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();

        save_config_registry(&ctx, "myfile.txt", dir.path().join("does-not-exist.txt"));
        save_safe_package_registry(&ctx);

        let secret_file = dir.path().join("secret.txt");
        fs::write(&secret_file, b"encrypted content").unwrap();
        let mut entries = HashMap::new();
        entries.insert(
            "secret".to_string(),
            EncryptedRegistryEntry {
                name: "Secret".to_string(),
                description: None,
                enabled: true,
                backup_path: "secret.txt".to_string(),
                original_path: secret_file,
            },
        );
        let registry = EncryptedRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.encrypted_registry_path()).unwrap();

        let password = SecretString::from("pw".to_string());
        let report = run(&ctx, &profile, Some(&password)).unwrap();

        let encrypted = report.encrypted.expect("encrypted section expected");
        assert_eq!(encrypted.succeeded(), 1);

        let bundle = profile
            .encrypted_backup_path(&ctx)
            .join(crate::context::ENCRYPTED_BUNDLE_FILE);
        assert!(bundle.exists());
    }
}
