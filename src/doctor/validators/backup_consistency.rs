use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use age::secrecy::SecretString;

use super::Validator;
use crate::context::Dfm;
use crate::doctor::report::ValidationError;
use crate::encryption::{
    collect_dir_tar_entries, create_temp_file, decrypt_file, load_tar_member_map,
};
use crate::profiles::ActiveProfile;
use crate::registry::{ConfigRegistry, EncryptedRegistry, EncryptedRegistryEntry};

const BACKUP_FIX: &str =
    "Run 'dfm backup' to update backup or 'dfm restore' to restore from backup";

/// Compares current files on disk against their backups (and, with a
/// password, encrypted backups) and warns on drift.
pub(super) struct BackupConsistencyValidator {
    ctx: Dfm,
    profile: ActiveProfile,
    password: Option<SecretString>,
}

impl BackupConsistencyValidator {
    pub(super) fn new(ctx: Dfm, profile: ActiveProfile, password: Option<SecretString>) -> Self {
        Self {
            ctx,
            profile,
            password,
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

        for (id, entry) in config_registry.get_enabled_entries() {
            if !entry.target_path.exists() || entry.target_path.is_dir() {
                continue;
            }

            let Some(resolved) = self.profile.resolve_source(ctx, &entry.source_path) else {
                continue;
            };
            if !resolved.path.exists() || resolved.path.is_dir() {
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
                match read_bytes(&entry.target_path, "current file", &entry.name, id) {
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
            .get_enabled_entries()
            .filter(|(_, e)| e.target_path.exists())
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

fn is_password_error(e: &impl std::fmt::Display) -> bool {
    let msg = e.to_string().to_lowercase();
    msg.contains("password") || msg.contains("decrypt") || msg.contains("identity")
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
        if is_password_error(&e) {
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

    let members = match load_tar_member_map(tar_temp.path()) {
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
        if entry.target_path.is_dir() {
            check_dir_entry(id, entry, &members, errors);
            continue;
        }

        let current_content = match read_bytes(&entry.target_path, "current file", &entry.name, id)
        {
            Ok(c) => c,
            Err(warning) => {
                errors.push(warning);
                continue;
            }
        };

        match members.get(&entry.source_path) {
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

/// Compare every file under a directory entry's `target_path` against its
/// matching `{source_path}/...` members in the decrypted bundle, and flag
/// any bundle members under that prefix missing on disk.
fn check_dir_entry(
    id: &str,
    entry: &EncryptedRegistryEntry,
    members: &HashMap<String, Vec<u8>>,
    errors: &mut Vec<ValidationError>,
) {
    let current_entries = match collect_dir_tar_entries(&entry.source_path, &entry.target_path) {
        Ok(entries) => entries,
        Err(e) => {
            errors.push(ValidationError::warning(format!(
                "Could not read directory {} for {} ({}): {}",
                entry.target_path.display(),
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

    let prefix = format!("{}/", entry.source_path);
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
