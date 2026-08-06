use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use age::secrecy::SecretString;

use super::Validator;
use crate::context::Dfm;
use crate::doctor::report::ValidationError;
use crate::encryption::{
    TarMemberMap, create_temp_file, decrypt_file, enumerate_tar_files, load_tar_files,
};
use crate::profiles::ActiveProfile;
use crate::registry::{
    ConfigRegistry, ConfigRegistryEntry, EncryptedRegistry, EncryptedRegistryEntry,
};

const BACKUP_FIX: &str =
    "Run 'dfm backup' to update backup or 'dfm restore' to restore from backup";

/// Compares current files on disk against their backups (and, with a
/// password, encrypted backups) and warns on drift.
pub(super) struct BackupConsistencyValidator {
    ctx: Dfm,
    profile: ActiveProfile,
    password: Option<SecretString>,
    include_disabled: bool,
}

impl BackupConsistencyValidator {
    /// `password` is only used to validate the encrypted bundle; when
    /// `None`, that half of the check is skipped. `include_disabled` also
    /// checks entries with `enabled: false` when `true`.
    pub(super) fn new(
        ctx: Dfm,
        profile: ActiveProfile,
        password: Option<SecretString>,
        include_disabled: bool,
    ) -> Self {
        Self {
            ctx,
            profile,
            password,
            include_disabled,
        }
    }
}

impl Validator for BackupConsistencyValidator {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let ctx = &self.ctx;

        let config_registry_path = ctx.config_registry_path();
        let config_registry = match ConfigRegistry::load_or_create(&config_registry_path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not load config registry: {}",
                    e
                )));
                return errors;
            }
        };

        for (id, entry) in config_registry.get_entries(self.include_disabled) {
            if !entry.original_path.exists() {
                continue;
            }

            let Some(resolved) = self.profile.resolve_backup_path(ctx, &entry.backup_path) else {
                continue;
            };
            if !resolved.path.exists() {
                continue;
            }

            if entry.original_path.is_dir() || resolved.path.is_dir() {
                check_plain_dir_entry(id, entry, &resolved.path, &mut errors);
                continue;
            }

            let backup_content = match read_bytes(&resolved.path, "backup file", &entry.name, id) {
                Ok(c) => c,
                Err(warning) => {
                    errors.push(warning);
                    continue;
                }
            };
            let current_content =
                match read_bytes(&entry.original_path, "current file", &entry.name, id) {
                    Ok(c) => c,
                    Err(warning) => {
                        errors.push(warning);
                        continue;
                    }
                };

            if let Some(warning) =
                report_if_differs(&current_content, &backup_content, &entry.name, id, "File")
            {
                errors.push(warning);
            }
        }

        let Some(password) = &self.password else {
            return errors;
        };

        let encrypted_registry_path = ctx.encrypted_registry_path();
        let encrypted_registry = match EncryptedRegistry::load_or_create(&encrypted_registry_path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not load encrypted config registry: {}",
                    e
                )));
                return errors;
            }
        };

        let encrypted_candidates: Vec<_> = encrypted_registry
            .get_entries(self.include_disabled)
            .filter(|(_, e)| e.original_path.exists())
            .map(|(id, e)| (id.clone(), e.clone()))
            .collect();

        if encrypted_candidates.is_empty() {
            return errors;
        }

        let Some(bundle) = self
            .profile
            .resolve_encrypted_bundle(ctx)
            .filter(|b| b.path.is_file())
        else {
            errors.push(ValidationError::warning(
                "no encrypted bundle backup found, skipping encrypted file validation".to_string(),
            ));
            return errors;
        };

        check_bundle(&bundle.path, password, &encrypted_candidates, &mut errors);
        errors
    }

    fn name(&self) -> &str {
        "Backup Consistency Check"
    }
}

/// Read `path`, turning any I/O error into a warning finding.
fn read_bytes(
    path: &Path,
    what: &str,
    entry_name: &str,
    id: &str,
) -> Result<Vec<u8>, ValidationError> {
    fs::read(path).map_err(|e| {
        ValidationError::warning(format!(
            "Could not read {} for {} ({}): {}",
            what, entry_name, id, e
        ))
    })
}

/// A warning finding if `current` and `backup` differ, `None` otherwise.
fn report_if_differs(
    current: &[u8],
    backup: &[u8],
    entry_name: &str,
    id: &str,
    label: &str,
) -> Option<ValidationError> {
    (current != backup).then(|| {
        ValidationError::warning(format!(
            "{} ({}): {} differs from backup",
            entry_name, id, label
        ))
        .with_fix(BACKUP_FIX)
    })
}

/// Compare every file under a directory config entry's `original_path`
/// against its counterpart in the plaintext backup layer, recursively, and
/// flag files present on only one side. A read failure on one file is
/// reported as a warning and does not stop the rest of the comparison.
fn check_plain_dir_entry(
    id: &str,
    entry: &ConfigRegistryEntry,
    backup_dir: &Path,
    errors: &mut Vec<ValidationError>,
) {
    let current_entries = match enumerate_tar_files(&entry.backup_path, &entry.original_path) {
        Ok(entries) => entries,
        Err(e) => {
            errors.push(ValidationError::warning(format!(
                "Could not read directory {} for {} ({}): {}",
                entry.original_path.display(),
                entry.name,
                id,
                e
            )));
            return;
        }
    };
    let backup_entries = match enumerate_tar_files(&entry.backup_path, backup_dir) {
        Ok(entries) => entries,
        Err(e) => {
            errors.push(ValidationError::warning(format!(
                "Could not read backup directory {} for {} ({}): {}",
                backup_dir.display(),
                entry.name,
                id,
                e
            )));
            return;
        }
    };

    let backup_by_name: HashMap<&str, &Path> = backup_entries
        .iter()
        .map(|(name, path)| (name.as_str(), path.as_path()))
        .collect();

    let mut seen = HashSet::with_capacity(current_entries.len());

    for (member_name, path) in &current_entries {
        seen.insert(member_name.as_str());

        let current_content = match read_bytes(path, "current file", &entry.name, id) {
            Ok(c) => c,
            Err(warning) => {
                errors.push(warning);
                continue;
            }
        };

        match backup_by_name.get(member_name.as_str()) {
            Some(backup_path) => {
                let backup_content = match read_bytes(backup_path, "backup file", &entry.name, id) {
                    Ok(c) => c,
                    Err(warning) => {
                        errors.push(warning);
                        continue;
                    }
                };
                if let Some(warning) =
                    report_if_differs(&current_content, &backup_content, &entry.name, id, "File")
                {
                    errors.push(warning);
                }
            }
            None => errors.push(
                ValidationError::warning(format!(
                    "{} ({}): {} missing from backup",
                    entry.name, id, member_name
                ))
                .with_fix(BACKUP_FIX),
            ),
        }
    }

    for (member_name, _) in &backup_entries {
        if !seen.contains(member_name.as_str()) {
            errors.push(
                ValidationError::warning(format!(
                    "{} ({}): {} present in backup but missing on disk",
                    entry.name, id, member_name
                ))
                .with_fix(BACKUP_FIX),
            );
        }
    }
}

/// Validate encrypted entries against the bundle backup.
fn check_bundle(
    bundle_path: &Path,
    password: &SecretString,
    candidates: &[(String, EncryptedRegistryEntry)],
    errors: &mut Vec<ValidationError>,
) {
    let tar_temp = match create_temp_file("validate-bundle-tar") {
        Ok(f) => f,
        Err(e) => {
            errors.push(ValidationError::warning(format!(
                "Could not create temporary file for bundle validation: {}",
                e
            )));
            return;
        }
    };

    if let Err(e) = decrypt_file(bundle_path, tar_temp.path(), password) {
        if e.is_password_error() {
            errors.push(ValidationError::warning(
                "Skipping encrypted file validation: Incorrect password".to_string(),
            ));
        } else {
            errors.push(ValidationError::warning(format!(
                "Could not decrypt bundle: {}",
                e
            )));
        }
        return;
    }

    let members = match load_tar_files(tar_temp.path()) {
        Ok(members) => members,
        Err(e) => {
            errors.push(ValidationError::warning(format!(
                "Could not read encrypted bundle: {}",
                e
            )));
            return;
        }
    };

    for (id, entry) in candidates {
        if entry.original_path.is_dir() {
            check_dir_entry(id, entry, &members, errors);
            continue;
        }

        let current_content =
            match read_bytes(&entry.original_path, "current file", &entry.name, id) {
                Ok(c) => c,
                Err(warning) => {
                    errors.push(warning);
                    continue;
                }
            };

        match members.get(&entry.backup_path) {
            Some(backup_content) => {
                if let Some(warning) = report_if_differs(
                    &current_content,
                    backup_content,
                    &entry.name,
                    id,
                    "Encrypted file",
                ) {
                    errors.push(warning);
                }
            }
            None => errors.push(ValidationError::warning(format!(
                "{} ({}): Missing from encrypted bundle backup",
                entry.name, id
            ))),
        }
    }
}

/// Compare every file under a directory entry's `original_path` against its
/// matching `{backup_path}/...` members in the decrypted bundle, and flag
/// any bundle members under that prefix missing on disk.
fn check_dir_entry(
    id: &str,
    entry: &EncryptedRegistryEntry,
    members: &TarMemberMap,
    errors: &mut Vec<ValidationError>,
) {
    let current_entries = match enumerate_tar_files(&entry.backup_path, &entry.original_path) {
        Ok(entries) => entries,
        Err(e) => {
            errors.push(ValidationError::warning(format!(
                "Could not read directory {} for {} ({}): {}",
                entry.original_path.display(),
                entry.name,
                id,
                e
            )));
            return;
        }
    };

    let mut seen = HashSet::with_capacity(current_entries.len());

    for (member_name, path) in &current_entries {
        seen.insert(member_name.as_str());

        let current_content = match read_bytes(path, "current file", &entry.name, id) {
            Ok(c) => c,
            Err(warning) => {
                errors.push(warning);
                continue;
            }
        };

        match members.get(member_name) {
            Some(backup_content) => {
                if let Some(warning) = report_if_differs(
                    &current_content,
                    backup_content,
                    &entry.name,
                    id,
                    "Encrypted file",
                ) {
                    errors.push(warning);
                }
            }
            None => errors.push(
                ValidationError::warning(format!(
                    "{} ({}): {} missing from encrypted bundle backup",
                    entry.name, id, member_name
                ))
                .with_fix(BACKUP_FIX),
            ),
        }
    }

    let prefix = format!("{}/", entry.backup_path);
    for member_name in members.keys() {
        if member_name.starts_with(&prefix) && !seen.contains(member_name.as_str()) {
            errors.push(
                ValidationError::warning(format!(
                    "{} ({}): {} present in encrypted bundle backup but missing on disk",
                    entry.name, id, member_name
                ))
                .with_fix(BACKUP_FIX),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::context::{Dfm, ENCRYPTED_BUNDLE_FILE};
    use crate::encryption::{TarEntryRef, encrypt_file, write_tar_archive};
    use crate::registry::EncryptedRegistry;

    use super::*;

    fn empty_config_registry(ctx: &Dfm) {
        let registry = ConfigRegistry {
            version: "1.0.0".to_string(),
            entries: HashMap::new(),
        };
        registry.save(&ctx.config_registry_path()).unwrap();
    }

    fn write_config_entry(ctx: &Dfm, id: &str, backup_path: &str, original_path: &Path) {
        write_config_entry_with_enabled(ctx, id, backup_path, original_path, true);
    }

    fn write_config_entry_with_enabled(
        ctx: &Dfm,
        id: &str,
        backup_path: &str,
        original_path: &Path,
        enabled: bool,
    ) {
        let mut entries = HashMap::new();
        entries.insert(
            id.to_string(),
            ConfigRegistryEntry {
                name: "Test Config".to_string(),
                description: None,
                enabled,
                backup_path: backup_path.to_string(),
                original_path: original_path.to_path_buf(),
            },
        );
        let registry = ConfigRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.config_registry_path()).unwrap();
    }

    #[test]
    fn name_is_backup_consistency_check() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        assert_eq!(validator.name(), "Backup Consistency Check");
    }

    #[test]
    fn plain_file_in_sync_produces_no_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let original_path = dir.path().join("home/.bashrc");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"export PATH=$PATH").unwrap();

        let backup_path = ctx.common_dir().join(".bashrc");
        fs::create_dir_all(backup_path.parent().unwrap()).unwrap();
        fs::write(&backup_path, b"export PATH=$PATH").unwrap();

        write_config_entry(&ctx, "bashrc", ".bashrc", &original_path);

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        let errors = validator.validate();

        assert!(errors.is_empty());
    }

    #[test]
    fn plain_file_differs_produces_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let original_path = dir.path().join("home/.bashrc");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"current content").unwrap();

        let backup_path = ctx.common_dir().join(".bashrc");
        fs::create_dir_all(backup_path.parent().unwrap()).unwrap();
        fs::write(&backup_path, b"stale backup content").unwrap();

        write_config_entry(&ctx, "bashrc", ".bashrc", &original_path);

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Test Config (bashrc): File differs from backup"
        );
        assert!(errors[0].fix_suggestion.is_some());
    }

    #[test]
    fn missing_original_path_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let original_path = dir.path().join("home/.bashrc");

        let backup_path = ctx.common_dir().join(".bashrc");
        fs::create_dir_all(backup_path.parent().unwrap()).unwrap();
        fs::write(&backup_path, b"backup content").unwrap();

        write_config_entry(&ctx, "bashrc", ".bashrc", &original_path);

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        let errors = validator.validate();

        assert!(errors.is_empty());
    }

    #[test]
    fn disabled_entry_is_skipped_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let original_path = dir.path().join("home/.bashrc");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"current content").unwrap();

        let backup_path = ctx.common_dir().join(".bashrc");
        fs::create_dir_all(backup_path.parent().unwrap()).unwrap();
        fs::write(&backup_path, b"stale backup content").unwrap();

        write_config_entry_with_enabled(&ctx, "bashrc", ".bashrc", &original_path, false);

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        let errors = validator.validate();

        assert!(errors.is_empty());
    }

    #[test]
    fn disabled_entry_is_checked_when_include_disabled_is_true() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let original_path = dir.path().join("home/.bashrc");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"current content").unwrap();

        let backup_path = ctx.common_dir().join(".bashrc");
        fs::create_dir_all(backup_path.parent().unwrap()).unwrap();
        fs::write(&backup_path, b"stale backup content").unwrap();

        write_config_entry_with_enabled(&ctx, "bashrc", ".bashrc", &original_path, false);

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, true);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Test Config (bashrc): File differs from backup"
        );
    }

    #[test]
    fn dir_entry_in_sync_produces_no_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let original_dir = dir.path().join("home/.config/nvim");
        fs::create_dir_all(&original_dir).unwrap();
        fs::write(original_dir.join("init.lua"), b"vim.opt.number = true").unwrap();

        let backup_dir = ctx.common_dir().join("nvim");
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(backup_dir.join("init.lua"), b"vim.opt.number = true").unwrap();

        write_config_entry(&ctx, "nvim", "nvim", &original_dir);

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        let errors = validator.validate();

        assert!(errors.is_empty());
    }

    #[test]
    fn dir_entry_modified_file_produces_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let original_dir = dir.path().join("home/.config/nvim");
        fs::create_dir_all(&original_dir).unwrap();
        fs::write(original_dir.join("init.lua"), b"current lua content").unwrap();

        let backup_dir = ctx.common_dir().join("nvim");
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(backup_dir.join("init.lua"), b"stale lua content").unwrap();

        write_config_entry(&ctx, "nvim", "nvim", &original_dir);

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Test Config (nvim): File differs from backup"
        );
    }

    #[test]
    fn dir_entry_file_missing_from_backup_produces_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let original_dir = dir.path().join("home/.config/nvim");
        fs::create_dir_all(&original_dir).unwrap();
        fs::write(original_dir.join("init.lua"), b"content").unwrap();
        fs::write(original_dir.join("extra.lua"), b"only on disk").unwrap();

        let backup_dir = ctx.common_dir().join("nvim");
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(backup_dir.join("init.lua"), b"content").unwrap();

        write_config_entry(&ctx, "nvim", "nvim", &original_dir);

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Test Config (nvim): nvim/extra.lua missing from backup"
        );
    }

    #[test]
    fn dir_entry_file_missing_on_disk_produces_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let original_dir = dir.path().join("home/.config/nvim");
        fs::create_dir_all(&original_dir).unwrap();
        fs::write(original_dir.join("init.lua"), b"content").unwrap();

        let backup_dir = ctx.common_dir().join("nvim");
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(backup_dir.join("init.lua"), b"content").unwrap();
        fs::write(backup_dir.join("old.lua"), b"only in backup").unwrap();

        write_config_entry(&ctx, "nvim", "nvim", &original_dir);

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Test Config (nvim): nvim/old.lua present in backup but missing on disk"
        );
    }

    #[test]
    fn no_password_skips_encrypted_check_entirely() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);

        let original_path = dir.path().join("home/.ssh/config");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"ssh config content").unwrap();

        let mut entries = HashMap::new();
        entries.insert(
            "ssh_config".to_string(),
            EncryptedRegistryEntry {
                name: "SSH Config".to_string(),
                description: None,
                enabled: true,
                backup_path: "ssh/config".to_string(),
                original_path: original_path.clone(),
            },
        );
        let encrypted_registry = EncryptedRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        encrypted_registry
            .save(&ctx.encrypted_registry_path())
            .unwrap();

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        let errors = validator.validate();

        assert!(errors.is_empty());
    }

    #[test]
    fn encrypted_bundle_wrong_password_reports_typed_incorrect_password_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);

        let original_path = dir.path().join("home/.ssh/config");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"ssh config content").unwrap();

        let mut entries = HashMap::new();
        entries.insert(
            "ssh_config".to_string(),
            EncryptedRegistryEntry {
                name: "SSH Config".to_string(),
                description: None,
                enabled: true,
                backup_path: "ssh/config".to_string(),
                original_path: original_path.clone(),
            },
        );
        let encrypted_registry = EncryptedRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        encrypted_registry
            .save(&ctx.encrypted_registry_path())
            .unwrap();

        let tar_path = dir.path().join("bundle.tar");
        write_tar_archive(&tar_path, &[("ssh/config", &original_path)]).unwrap();

        let bundle_path = ctx.encrypted_common_dir().join(ENCRYPTED_BUNDLE_FILE);
        let correct_password = SecretString::from("correct horse battery staple".to_string());
        encrypt_file(&tar_path, &bundle_path, &correct_password).unwrap();

        let wrong_password = SecretString::from("totally wrong password".to_string());
        let validator = BackupConsistencyValidator::new(
            ctx,
            ActiveProfile::common_only(),
            Some(wrong_password),
            false,
        );
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Skipping encrypted file validation: Incorrect password"
        );
    }

    fn write_encrypted_entry(
        ctx: &Dfm,
        id: &str,
        backup_path: &str,
        original_path: &Path,
    ) -> EncryptedRegistryEntry {
        let entry = EncryptedRegistryEntry {
            name: "Test Encrypted".to_string(),
            description: None,
            enabled: true,
            backup_path: backup_path.to_string(),
            original_path: original_path.to_path_buf(),
        };
        let mut entries = HashMap::new();
        entries.insert(id.to_string(), entry.clone());
        let registry = EncryptedRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.encrypted_registry_path()).unwrap();
        entry
    }

    /// Build and encrypt a bundle from `(archive name, original file)` pairs
    /// via the same tar+encrypt helpers the real backup path uses.
    fn write_encrypted_bundle(
        ctx: &Dfm,
        profile: &ActiveProfile,
        entries: &[TarEntryRef],
        password: &SecretString,
    ) {
        let tar_temp = create_temp_file("test-bundle-build").unwrap();
        write_tar_archive(tar_temp.path(), entries).unwrap();
        let bundle_dir = profile.encrypted_backup_path(ctx);
        fs::create_dir_all(&bundle_dir).unwrap();
        encrypt_file(
            tar_temp.path(),
            &bundle_dir.join(ENCRYPTED_BUNDLE_FILE),
            password,
        )
        .unwrap();
    }

    #[test]
    fn encrypted_dir_entry_in_sync_produces_no_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);
        let profile = ActiveProfile::common_only();

        let original_dir = dir.path().join("home/.ssh/keys");
        fs::create_dir_all(&original_dir).unwrap();
        fs::write(original_dir.join("id_rsa"), b"private key content").unwrap();

        write_encrypted_entry(&ctx, "ssh_keys", "ssh/keys", &original_dir);

        let password = SecretString::from("pw".to_string());
        write_encrypted_bundle(
            &ctx,
            &profile,
            &[("ssh/keys/id_rsa", original_dir.join("id_rsa").as_path())],
            &password,
        );

        let validator = BackupConsistencyValidator::new(ctx, profile, Some(password), false);
        let errors = validator.validate();

        assert!(errors.is_empty());
    }

    #[test]
    fn encrypted_dir_entry_modified_file_produces_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);
        let profile = ActiveProfile::common_only();

        let original_dir = dir.path().join("home/.ssh/keys");
        fs::create_dir_all(&original_dir).unwrap();
        fs::write(original_dir.join("id_rsa"), b"current key content").unwrap();

        write_encrypted_entry(&ctx, "ssh_keys", "ssh/keys", &original_dir);

        // Build the bundle from a different original file so the archived
        // content differs from what's currently on disk.
        let stale_original = dir.path().join("stale_id_rsa");
        fs::write(&stale_original, b"stale key content").unwrap();

        let password = SecretString::from("pw".to_string());
        write_encrypted_bundle(
            &ctx,
            &profile,
            &[("ssh/keys/id_rsa", stale_original.as_path())],
            &password,
        );

        let validator = BackupConsistencyValidator::new(ctx, profile, Some(password), false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Test Encrypted (ssh_keys): Encrypted file differs from backup"
        );
    }

    #[test]
    fn encrypted_dir_entry_file_missing_from_bundle_produces_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);
        let profile = ActiveProfile::common_only();

        let original_dir = dir.path().join("home/.ssh/keys");
        fs::create_dir_all(&original_dir).unwrap();
        fs::write(original_dir.join("id_rsa"), b"key content").unwrap();
        fs::write(original_dir.join("id_rsa.pub"), b"only on disk").unwrap();

        write_encrypted_entry(&ctx, "ssh_keys", "ssh/keys", &original_dir);

        let password = SecretString::from("pw".to_string());
        write_encrypted_bundle(
            &ctx,
            &profile,
            &[("ssh/keys/id_rsa", original_dir.join("id_rsa").as_path())],
            &password,
        );

        let validator = BackupConsistencyValidator::new(ctx, profile, Some(password), false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Test Encrypted (ssh_keys): ssh/keys/id_rsa.pub missing from encrypted bundle backup"
        );
    }

    #[test]
    fn encrypted_dir_entry_file_missing_on_disk_produces_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);
        let profile = ActiveProfile::common_only();

        let original_dir = dir.path().join("home/.ssh/keys");
        fs::create_dir_all(&original_dir).unwrap();
        fs::write(original_dir.join("id_rsa"), b"key content").unwrap();

        write_encrypted_entry(&ctx, "ssh_keys", "ssh/keys", &original_dir);

        // The bundle has an extra member under the same prefix that isn't
        // present on disk.
        let extra_source = dir.path().join("id_rsa.pub");
        fs::write(&extra_source, b"only in backup").unwrap();

        let password = SecretString::from("pw".to_string());
        write_encrypted_bundle(
            &ctx,
            &profile,
            &[
                ("ssh/keys/id_rsa", original_dir.join("id_rsa").as_path()),
                ("ssh/keys/id_rsa.pub", extra_source.as_path()),
            ],
            &password,
        );

        let validator = BackupConsistencyValidator::new(ctx, profile, Some(password), false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Test Encrypted (ssh_keys): ssh/keys/id_rsa.pub present in encrypted bundle backup but missing on disk"
        );
    }

    #[test]
    fn encrypted_bundle_generic_decrypt_failure_reports_generic_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);
        let profile = ActiveProfile::common_only();

        let original_path = dir.path().join("home/.ssh/config");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"ssh config content").unwrap();

        write_encrypted_entry(&ctx, "ssh_config", "ssh/config", &original_path);

        // Not valid age ciphertext at all, so decryption fails for a reason
        // other than an incorrect password.
        let bundle_dir = profile.encrypted_backup_path(&ctx);
        fs::create_dir_all(&bundle_dir).unwrap();
        fs::write(
            bundle_dir.join(ENCRYPTED_BUNDLE_FILE),
            b"not age ciphertext, just garbage bytes",
        )
        .unwrap();

        let password = SecretString::from("any password".to_string());
        let validator = BackupConsistencyValidator::new(ctx, profile, Some(password), false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.starts_with("Could not decrypt bundle:"));
    }

    #[test]
    fn encrypted_bundle_unreadable_tar_reports_could_not_read_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);
        let profile = ActiveProfile::common_only();

        let original_path = dir.path().join("home/.ssh/config");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"ssh config content").unwrap();

        write_encrypted_entry(&ctx, "ssh_config", "ssh/config", &original_path);

        // The bundle decrypts fine, but the decrypted payload isn't a valid
        // tar archive.
        let not_a_tar = dir.path().join("not-a-tar");
        fs::write(&not_a_tar, b"not a tar file at all, just garbage bytes").unwrap();

        let password = SecretString::from("pw".to_string());
        let bundle_dir = profile.encrypted_backup_path(&ctx);
        fs::create_dir_all(&bundle_dir).unwrap();
        encrypt_file(
            &not_a_tar,
            &bundle_dir.join(ENCRYPTED_BUNDLE_FILE),
            &password,
        )
        .unwrap();

        let validator = BackupConsistencyValidator::new(ctx, profile, Some(password), false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .message
                .starts_with("Could not read encrypted bundle:")
        );
    }

    #[test]
    fn invalid_config_registry_json_reports_error_and_returns_early() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        fs::create_dir_all(ctx.config_registry_path().parent().unwrap()).unwrap();
        fs::write(ctx.config_registry_path(), b"not valid json").unwrap();

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .message
                .starts_with("Could not load config registry:")
        );
    }

    #[test]
    fn invalid_encrypted_registry_json_reports_error_when_password_given() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);

        fs::create_dir_all(ctx.encrypted_registry_path().parent().unwrap()).unwrap();
        fs::write(ctx.encrypted_registry_path(), b"not valid json").unwrap();

        let password = SecretString::from("pw".to_string());
        let validator = BackupConsistencyValidator::new(
            ctx,
            ActiveProfile::common_only(),
            Some(password),
            false,
        );
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .message
                .starts_with("Could not load encrypted config registry:")
        );
    }

    #[test]
    fn no_encrypted_candidates_returns_no_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);

        // An enabled encrypted entry whose original doesn't exist on disk is
        // filtered out, leaving no candidates to validate.
        let original_path = dir.path().join("home/.ssh/config");
        write_encrypted_entry(&ctx, "ssh_config", "ssh/config", &original_path);

        let password = SecretString::from("pw".to_string());
        let validator = BackupConsistencyValidator::new(
            ctx,
            ActiveProfile::common_only(),
            Some(password),
            false,
        );
        let errors = validator.validate();

        assert!(errors.is_empty());
    }

    #[test]
    fn missing_resolved_backup_path_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        // original_path exists, but the resolved backup path under
        // common/profile layers does not, so this entry should be skipped
        // rather than reported.
        let original_path = dir.path().join("home/.bashrc");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"current content").unwrap();

        write_config_entry(&ctx, "bashrc", ".bashrc", &original_path);

        let validator =
            BackupConsistencyValidator::new(ctx, ActiveProfile::common_only(), None, false);
        let errors = validator.validate();

        assert!(errors.is_empty());
    }

    #[test]
    fn no_encrypted_bundle_found_reports_warning_when_candidates_exist() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);

        let original_path = dir.path().join("home/.ssh/config");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"ssh config content").unwrap();
        write_encrypted_entry(&ctx, "ssh_config", "ssh/config", &original_path);

        // No bundle file is ever written, so resolve_encrypted_bundle finds
        // nothing even though there's a candidate to validate.
        let password = SecretString::from("pw".to_string());
        let validator = BackupConsistencyValidator::new(
            ctx,
            ActiveProfile::common_only(),
            Some(password),
            false,
        );
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "no encrypted bundle backup found, skipping encrypted file validation"
        );
    }

    #[test]
    fn encrypted_single_file_in_sync_produces_no_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);
        let profile = ActiveProfile::common_only();

        let original_path = dir.path().join("home/.ssh/config");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"ssh config content").unwrap();
        write_encrypted_entry(&ctx, "ssh_config", "ssh/config", &original_path);

        let password = SecretString::from("pw".to_string());
        write_encrypted_bundle(
            &ctx,
            &profile,
            &[("ssh/config", original_path.as_path())],
            &password,
        );

        let validator = BackupConsistencyValidator::new(ctx, profile, Some(password), false);
        let errors = validator.validate();

        assert!(errors.is_empty());
    }

    #[test]
    fn encrypted_single_file_differs_produces_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);
        let profile = ActiveProfile::common_only();

        let original_path = dir.path().join("home/.ssh/config");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"current ssh config").unwrap();
        write_encrypted_entry(&ctx, "ssh_config", "ssh/config", &original_path);

        let stale_original = dir.path().join("stale_config");
        fs::write(&stale_original, b"stale ssh config").unwrap();

        let password = SecretString::from("pw".to_string());
        write_encrypted_bundle(
            &ctx,
            &profile,
            &[("ssh/config", stale_original.as_path())],
            &password,
        );

        let validator = BackupConsistencyValidator::new(ctx, profile, Some(password), false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Test Encrypted (ssh_config): Encrypted file differs from backup"
        );
    }

    #[test]
    fn encrypted_single_file_missing_from_bundle_produces_warning() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        empty_config_registry(&ctx);
        let profile = ActiveProfile::common_only();

        let original_path = dir.path().join("home/.ssh/config");
        fs::create_dir_all(original_path.parent().unwrap()).unwrap();
        fs::write(&original_path, b"ssh config content").unwrap();
        write_encrypted_entry(&ctx, "ssh_config", "ssh/config", &original_path);

        // Bundle exists, but has no member matching this entry's backup
        // path at all.
        let other_original = dir.path().join("other_secret");
        fs::write(&other_original, b"unrelated content").unwrap();

        let password = SecretString::from("pw".to_string());
        write_encrypted_bundle(
            &ctx,
            &profile,
            &[("other/secret", other_original.as_path())],
            &password,
        );

        let validator = BackupConsistencyValidator::new(ctx, profile, Some(password), false);
        let errors = validator.validate();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Test Encrypted (ssh_config): Missing from encrypted bundle backup"
        );
    }
}
