use std::fs;
use std::path::{Path, PathBuf};

use age::secrecy::SecretString;

use crate::context::{Dotfm, ENCRYPTED_BUNDLE_FILE};
use crate::encryption::{
    collect_dir_tar_entries, create_temp_file, encrypt_file, write_entries_tar,
};
use crate::error::{Result, WrapErr};
use crate::registry::EncryptedRegistry;
use crate::report::{ItemOutcome, SectionReport};

/// One registry entry's queued tar members: `(id, label, [(archive name, source file)])`.
type ArchiveEntry = (String, String, Vec<(String, PathBuf)>);

/// Tar every enabled encrypted registry entry that exists on disk and
/// encrypt it into a single bundle at `encrypted_backup_path`. Removes any
/// existing bundle when there's nothing to archive.
pub(super) fn backup_encrypted_configs(
    ctx: &Dotfm,
    encrypted_backup_path: &Path,
    password: &SecretString,
) -> Result<SectionReport> {
    let registry_path = ctx.encrypted_registry_path();
    let encrypted_registry = EncryptedRegistry::load_or_create(&registry_path)
        .wrap_err_with(|| format!("Load encrypted registry: {}", registry_path.display()))?;

    let enabled_entries: Vec<_> = encrypted_registry.get_enabled_entries().collect();

    let mut report = SectionReport::default();
    let mut to_archive: Vec<ArchiveEntry> = Vec::new();

    for (id, entry) in enabled_entries {
        if !entry.target_path.exists() {
            report.outcomes.push(ItemOutcome::skipped(
                id,
                &entry.source_path,
                format!("missing target {}", entry.target_path.display()),
            ));
            continue;
        }

        let members = if entry.target_path.is_dir() {
            match collect_dir_tar_entries(&entry.source_path, &entry.target_path) {
                Ok(members) => members,
                Err(e) => {
                    report.outcomes.push(ItemOutcome::skipped(
                        id,
                        &entry.source_path,
                        format!("could not read directory: {e}"),
                    ));
                    continue;
                }
            }
        } else {
            vec![(entry.source_path.clone(), entry.target_path.clone())]
        };

        to_archive.push((id.clone(), entry.source_path.clone(), members));
    }

    to_archive.sort_by(|a, b| a.1.cmp(&b.1));

    let bundle_destination = encrypted_backup_path.join(ENCRYPTED_BUNDLE_FILE);

    if to_archive.is_empty() {
        let _ = fs::remove_file(&bundle_destination);
        return Ok(report);
    }

    let tar_refs: Vec<(&str, &Path)> = to_archive
        .iter()
        .flat_map(|(_, _, members)| members.iter())
        .map(|(name, path)| (name.as_str(), path.as_path()))
        .collect();

    let tar_temp = create_temp_file("enc-bundle-tar").wrap_err("Create temporary tar path")?;

    write_entries_tar(tar_temp.path(), &tar_refs)?;
    encrypt_file(tar_temp.path(), &bundle_destination, password)
        .wrap_err("Encrypt config bundle")?;

    for (id, source_path, _) in &to_archive {
        report
            .outcomes
            .push(ItemOutcome::done(id.clone(), source_path.clone()));
    }

    Ok(report)
}
