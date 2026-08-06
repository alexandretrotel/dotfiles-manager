use std::fs;
use std::path::{Path, PathBuf};

use age::secrecy::SecretString;

use crate::context::{Dfm, ENCRYPTED_BUNDLE_FILE};
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
    ctx: &Dfm,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::{decrypt_file, load_tar_member_map};
    use crate::registry::EncryptedRegistryEntry;
    use std::collections::HashMap;

    fn entry(source_path: &str, target_path: PathBuf) -> EncryptedRegistryEntry {
        EncryptedRegistryEntry {
            name: source_path.to_string(),
            description: None,
            enabled: true,
            source_path: source_path.to_string(),
            target_path,
        }
    }

    fn save_registry(ctx: &Dfm, entries: HashMap<String, EncryptedRegistryEntry>) {
        let registry = EncryptedRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.encrypted_registry_path()).unwrap();
    }

    #[test]
    fn backs_up_a_file_entry_into_encrypted_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let target = dir.path().join("secret.txt");
        fs::write(&target, b"top secret").unwrap();

        let mut entries = HashMap::new();
        entries.insert("secret".to_string(), entry("secret.txt", target));
        save_registry(&ctx, entries);

        let encrypted_backup_path = dir.path().join("encrypted");
        fs::create_dir_all(&encrypted_backup_path).unwrap();
        let password = SecretString::from("test password".to_string());

        let report = backup_encrypted_configs(&ctx, &encrypted_backup_path, &password).unwrap();

        assert_eq!(report.succeeded(), 1);
        let bundle = encrypted_backup_path.join(ENCRYPTED_BUNDLE_FILE);
        assert!(bundle.exists());

        let tar_temp = create_temp_file("test-decrypt").unwrap();
        decrypt_file(&bundle, tar_temp.path(), &password).unwrap();
        let members = load_tar_member_map(tar_temp.path()).unwrap();
        assert_eq!(members.get("secret.txt").unwrap(), b"top secret");
    }

    #[test]
    fn missing_target_path_is_skipped_and_no_bundle_written() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let missing = dir.path().join("nope.txt");

        let mut entries = HashMap::new();
        entries.insert("missing".to_string(), entry("nope.txt", missing));
        save_registry(&ctx, entries);

        let encrypted_backup_path = dir.path().join("encrypted");
        fs::create_dir_all(&encrypted_backup_path).unwrap();
        let password = SecretString::from("pw".to_string());

        let report = backup_encrypted_configs(&ctx, &encrypted_backup_path, &password).unwrap();

        assert_eq!(report.succeeded(), 0);
        assert_eq!(report.skipped(), 1);
        assert!(!encrypted_backup_path.join(ENCRYPTED_BUNDLE_FILE).exists());
    }

    #[test]
    fn backs_up_a_directory_entry_with_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let target = dir.path().join("secrets_dir");
        fs::create_dir_all(target.join("nested")).unwrap();
        fs::write(target.join("a.txt"), b"a").unwrap();
        fs::write(target.join("nested/b.txt"), b"b").unwrap();

        let mut entries = HashMap::new();
        entries.insert("dir".to_string(), entry("secrets", target));
        save_registry(&ctx, entries);

        let encrypted_backup_path = dir.path().join("encrypted");
        fs::create_dir_all(&encrypted_backup_path).unwrap();
        let password = SecretString::from("pw".to_string());

        let report = backup_encrypted_configs(&ctx, &encrypted_backup_path, &password).unwrap();
        assert_eq!(report.succeeded(), 1);

        let bundle = encrypted_backup_path.join(ENCRYPTED_BUNDLE_FILE);
        let tar_temp = create_temp_file("test-decrypt-dir").unwrap();
        decrypt_file(&bundle, tar_temp.path(), &password).unwrap();
        let members = load_tar_member_map(tar_temp.path()).unwrap();
        assert_eq!(members.get("secrets/a.txt").unwrap(), b"a");
        assert_eq!(members.get("secrets/nested/b.txt").unwrap(), b"b");
    }
}
