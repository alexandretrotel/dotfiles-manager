use std::collections::HashMap;
use std::fs;

use age::secrecy::SecretString;

use crate::context::Dotfm;
use crate::encryption::{
    create_temp_file, decrypt_file, load_tar_member_map, set_private_file_permissions,
};
use crate::profiles::ActiveProfile;
use crate::registry::{EncryptedRegistry, EncryptedRegistryEntry};
use crate::report::{ItemOutcome, SectionReport};

/// Restore every enabled encrypted registry entry from the encrypted bundle.
pub(super) fn restore_encrypted_configs(
    ctx: &Dotfm,
    profile: &ActiveProfile,
    password: &SecretString,
) -> SectionReport {
    let mut report = SectionReport::default();

    let encrypted_registry_path = ctx.encrypted_registry_path();
    let encrypted_registry = match EncryptedRegistry::load_or_create(&encrypted_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            report.warnings.push(format!(
                "failed to load encrypted registry, skipping encrypted restore: {}",
                e
            ));
            return report;
        }
    };

    let enabled_entries: Vec<_> = encrypted_registry
        .get_enabled_entries()
        .map(|(id, e)| (id.clone(), e.clone()))
        .collect();

    if enabled_entries.is_empty() {
        return report;
    }

    let Some(bundle) = profile
        .resolve_encrypted_bundle(ctx)
        .filter(|b| b.path.is_file())
    else {
        report
            .warnings
            .push("no encrypted bundle backup found, skipping encrypted restore".to_string());
        return report;
    };

    let tar_temp = match create_temp_file("enc-restore-tar") {
        Ok(f) => f,
        Err(e) => {
            report.warnings.push(format!(
                "could not create temp file for bundle restore: {}",
                e
            ));
            return report;
        }
    };

    match decrypt_file(&bundle.path, tar_temp.path(), password) {
        Ok(()) => match load_tar_member_map(tar_temp.path()) {
            Ok(members) => restore_from_bundle_members(&enabled_entries, &members, &mut report),
            Err(e) => report
                .warnings
                .push(format!("could not read encrypted bundle archive: {}", e)),
        },
        Err(e) => report.warnings.push(format!(
            "could not decrypt {}: {}",
            bundle.path.display(),
            e
        )),
    }

    report
}

/// Write each enabled entry's contents from the decrypted bundle's member
/// map to its target path.
fn restore_from_bundle_members(
    enabled_entries: &[(String, EncryptedRegistryEntry)],
    members: &HashMap<String, Vec<u8>>,
    report: &mut SectionReport,
) {
    for (id, entry) in enabled_entries {
        let target_path = &entry.target_path;

        let outcome = match members.get(&entry.source_path) {
            Some(contents) => (|| {
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        format!("failed to create directory {}: {}", parent.display(), e)
                    })?;
                }

                fs::write(target_path, contents)
                    .map_err(|e| format!("failed to write {}: {}", target_path.display(), e))?;

                match set_private_file_permissions(target_path) {
                    Ok(()) => Ok(None),
                    Err(e) => Ok(Some(format!(
                        "restored, but failed to set permissions on {}: {}",
                        target_path.display(),
                        e
                    ))),
                }
            })()
            .map_or_else(
                |reason: String| ItemOutcome::skipped(id, &entry.source_path, reason),
                |note: Option<String>| match note {
                    Some(note) => ItemOutcome::done_with_note(id, &entry.source_path, note),
                    None => ItemOutcome::done(id, &entry.source_path),
                },
            ),
            None => ItemOutcome::skipped(id, &entry.source_path, "not in encrypted bundle"),
        };

        report.outcomes.push(outcome);
    }
}
