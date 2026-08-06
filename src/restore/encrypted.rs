use std::fs;
use std::path::Path;

use age::secrecy::SecretString;

use crate::context::Dfm;
use crate::encryption::{
    TarMemberMap, create_temp_file, decrypt_file, load_tar_files, set_private_file_permissions,
};
use crate::profiles::ActiveProfile;
use crate::registry::{EncryptedRegistry, EncryptedRegistryEntry};
use crate::report::{RegistryEntryOutcome, SectionReport};

/// Restore every enabled encrypted registry entry from the encrypted bundle.
pub(super) fn restore_encrypted_configs(
    ctx: &Dfm,
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
        Ok(()) => match load_tar_files(tar_temp.path()) {
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
/// map to its target path. A `source_path` matching a member exactly is
/// restored as a single file; one only matching as a `source_path/`
/// prefix is restored as a directory tree.
fn restore_from_bundle_members(
    enabled_entries: &[(String, EncryptedRegistryEntry)],
    members: &TarMemberMap,
    report: &mut SectionReport,
) {
    for (id, entry) in enabled_entries {
        let target_path = &entry.target_path;

        let result = if members.contains_key(&entry.source_path) {
            restore_file(target_path, &members[&entry.source_path])
        } else {
            restore_dir(target_path, &entry.source_path, members)
        };

        let outcome = result.map_or_else(
            |reason| RegistryEntryOutcome::skipped(id, &entry.source_path, reason),
            |note| match note {
                Some(note) => RegistryEntryOutcome::done_with_note(id, &entry.source_path, note),
                None => RegistryEntryOutcome::done(id, &entry.source_path),
            },
        );

        report.outcomes.push(outcome);
    }
}

/// Write a single file entry's contents to `target_path`.
fn restore_file(target_path: &Path, contents: &[u8]) -> Result<Option<String>, String> {
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directory {}: {}", parent.display(), e))?;
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
}

/// Write every bundle member prefixed `{source_prefix}/` into `target_path`,
/// recreating the relative directory structure.
fn restore_dir(
    target_path: &Path,
    source_prefix: &str,
    members: &TarMemberMap,
) -> Result<Option<String>, String> {
    let prefix = format!("{source_prefix}/");
    let mut written = 0usize;
    let mut warnings = Vec::new();

    for (member_path, contents) in members {
        let Some(relative) = member_path.strip_prefix(&prefix) else {
            continue;
        };
        let dest = target_path.join(relative);

        if let Err(e) = fs::create_dir_all(dest.parent().unwrap_or(target_path)) {
            warnings.push(format!(
                "failed to create directory for {}: {}",
                dest.display(),
                e
            ));
            continue;
        }
        if let Err(e) = fs::write(&dest, contents) {
            warnings.push(format!("failed to write {}: {}", dest.display(), e));
            continue;
        }
        if let Err(e) = set_private_file_permissions(&dest) {
            warnings.push(format!(
                "failed to set permissions on {}: {}",
                dest.display(),
                e
            ));
        }
        written += 1;
    }

    if written == 0 && warnings.is_empty() {
        return Err("not in encrypted bundle".to_string());
    }

    Ok((!warnings.is_empty()).then(|| warnings.join("; ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ENCRYPTED_BUNDLE_FILE;
    use crate::encryption::{encrypt_file, write_tar_archive};
    use std::collections::HashMap;
    use std::path::PathBuf;

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
    fn restores_a_file_member_from_the_encrypted_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();
        let password = SecretString::from("pw".to_string());

        let target = dir.path().join("restored.txt");
        let mut entries = HashMap::new();
        entries.insert("secret".to_string(), entry("secret.txt", target.clone()));
        save_registry(&ctx, entries);

        let source_file = dir.path().join("plain-secret.txt");
        fs::write(&source_file, b"top secret").unwrap();
        let bundle_dir = profile.encrypted_backup_path(&ctx);
        fs::create_dir_all(&bundle_dir).unwrap();
        let tar_temp = create_temp_file("test-bundle").unwrap();
        write_tar_archive(tar_temp.path(), &[("secret.txt", source_file.as_path())]).unwrap();
        encrypt_file(
            tar_temp.path(),
            &bundle_dir.join(ENCRYPTED_BUNDLE_FILE),
            &password,
        )
        .unwrap();

        let report = restore_encrypted_configs(&ctx, &profile, &password);

        assert_eq!(report.succeeded(), 1);
        assert_eq!(fs::read(&target).unwrap(), b"top secret");
    }

    #[test]
    fn no_bundle_backup_found_produces_warning_not_error() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();
        let password = SecretString::from("pw".to_string());

        let mut entries = HashMap::new();
        entries.insert(
            "secret".to_string(),
            entry("secret.txt", dir.path().join("target.txt")),
        );
        save_registry(&ctx, entries);

        let report = restore_encrypted_configs(&ctx, &profile, &password);

        assert!(report.outcomes.is_empty());
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("no encrypted bundle backup found"));
    }

    #[test]
    fn empty_registry_returns_empty_report_without_looking_for_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();
        let password = SecretString::from("pw".to_string());

        save_registry(&ctx, HashMap::new());

        let report = restore_encrypted_configs(&ctx, &profile, &password);

        assert!(report.is_empty());
    }

    #[test]
    fn restore_dir_writes_nested_files_from_matching_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target_dir");

        let mut members = HashMap::new();
        members.insert("mydir/a.txt".to_string(), b"a".to_vec());
        members.insert("mydir/sub/b.txt".to_string(), b"b".to_vec());
        members.insert("otherdir/c.txt".to_string(), b"c".to_vec());

        let note = restore_dir(&target, "mydir", &members).unwrap();
        assert!(note.is_none());

        assert_eq!(fs::read(target.join("a.txt")).unwrap(), b"a");
        assert_eq!(fs::read(target.join("sub/b.txt")).unwrap(), b"b");
        assert!(!target.join("c.txt").exists());
    }

    #[test]
    fn restore_dir_errors_when_prefix_not_in_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target_dir");
        let members: TarMemberMap = HashMap::new();

        let err = restore_dir(&target, "missing-prefix", &members).unwrap_err();
        assert_eq!(err, "not in encrypted bundle");
    }

    #[test]
    fn restore_dir_reports_note_when_a_member_path_collides_with_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target_dir");
        // Pre-create a regular file where a member needs a directory
        // (`sub`), so `fs::create_dir_all` for that member fails while the
        // rest of the tree still restores.
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("sub"), b"blocking file").unwrap();

        let mut members = HashMap::new();
        members.insert("mydir/ok.txt".to_string(), b"fine".to_vec());
        members.insert("mydir/sub/blocked.txt".to_string(), b"nope".to_vec());

        let note = restore_dir(&target, "mydir", &members).unwrap().unwrap();

        assert!(note.contains("failed to create directory for"));
        assert_eq!(fs::read(target.join("ok.txt")).unwrap(), b"fine");
        assert!(!target.join("sub/blocked.txt").exists());
    }

    #[test]
    fn restore_dir_reports_note_when_write_target_is_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target_dir");
        // Pre-create a directory where a member needs to write a plain
        // file, so `fs::write` for that member fails.
        fs::create_dir_all(target.join("blocked")).unwrap();

        let mut members = HashMap::new();
        members.insert("mydir/ok.txt".to_string(), b"fine".to_vec());
        members.insert("mydir/blocked".to_string(), b"nope".to_vec());

        let note = restore_dir(&target, "mydir", &members).unwrap().unwrap();

        assert!(note.contains("failed to write"));
        assert_eq!(fs::read(target.join("ok.txt")).unwrap(), b"fine");
    }

    #[test]
    fn restore_from_bundle_members_surfaces_a_partial_failure_as_done_with_note() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target_dir");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("sub"), b"blocking file").unwrap();

        let mut members = HashMap::new();
        members.insert("mydir/ok.txt".to_string(), b"fine".to_vec());
        members.insert("mydir/sub/blocked.txt".to_string(), b"nope".to_vec());

        let enabled_entries = vec![("dir_id".to_string(), entry("mydir", target.clone()))];
        let mut report = SectionReport::default();

        restore_from_bundle_members(&enabled_entries, &members, &mut report);

        assert_eq!(report.outcomes.len(), 1);
        assert_eq!(report.succeeded(), 1);
        match &report.outcomes[0].status {
            crate::report::RegistryEntryStatus::Done { note: Some(note) } => {
                assert!(note.contains("failed to create directory for"));
            }
            other => panic!("expected Done with note, got {other:?}"),
        }
    }

    #[test]
    fn restores_a_directory_member_from_the_encrypted_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();
        let password = SecretString::from("pw".to_string());

        let target = dir.path().join("restored_dir");
        let mut entries = HashMap::new();
        entries.insert("ssh_keys".to_string(), entry("ssh/keys", target.clone()));
        save_registry(&ctx, entries);

        let source_a = dir.path().join("id_rsa");
        let source_b = dir.path().join("id_rsa.pub");
        fs::write(&source_a, b"private key").unwrap();
        fs::write(&source_b, b"public key").unwrap();

        let bundle_dir = profile.encrypted_backup_path(&ctx);
        fs::create_dir_all(&bundle_dir).unwrap();
        let tar_temp = create_temp_file("test-bundle-dir").unwrap();
        write_tar_archive(
            tar_temp.path(),
            &[
                ("ssh/keys/id_rsa", source_a.as_path()),
                ("ssh/keys/id_rsa.pub", source_b.as_path()),
            ],
        )
        .unwrap();
        encrypt_file(
            tar_temp.path(),
            &bundle_dir.join(ENCRYPTED_BUNDLE_FILE),
            &password,
        )
        .unwrap();

        let report = restore_encrypted_configs(&ctx, &profile, &password);

        assert_eq!(report.succeeded(), 1);
        assert_eq!(fs::read(target.join("id_rsa")).unwrap(), b"private key");
        assert_eq!(fs::read(target.join("id_rsa.pub")).unwrap(), b"public key");
    }

    #[test]
    fn entry_not_present_in_bundle_is_skipped_not_a_hard_error() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();
        let password = SecretString::from("pw".to_string());

        let target = dir.path().join("restored.txt");
        let mut entries = HashMap::new();
        entries.insert(
            "missing".to_string(),
            entry("not-in-bundle.txt", target.clone()),
        );
        save_registry(&ctx, entries);

        // Bundle exists but has no member matching the entry's source path.
        let other_source = dir.path().join("other.txt");
        fs::write(&other_source, b"unrelated content").unwrap();
        let bundle_dir = profile.encrypted_backup_path(&ctx);
        fs::create_dir_all(&bundle_dir).unwrap();
        let tar_temp = create_temp_file("test-bundle-missing-member").unwrap();
        write_tar_archive(tar_temp.path(), &[("other.txt", other_source.as_path())]).unwrap();
        encrypt_file(
            tar_temp.path(),
            &bundle_dir.join(ENCRYPTED_BUNDLE_FILE),
            &password,
        )
        .unwrap();

        let report = restore_encrypted_configs(&ctx, &profile, &password);

        assert_eq!(report.succeeded(), 0);
        assert_eq!(report.skipped(), 1);
        assert!(!target.exists());
    }

    #[test]
    fn corrupt_bundle_that_fails_to_decrypt_produces_warning_not_hard_error() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();
        let password = SecretString::from("pw".to_string());

        let mut entries = HashMap::new();
        entries.insert(
            "secret".to_string(),
            entry("secret.txt", dir.path().join("target.txt")),
        );
        save_registry(&ctx, entries);

        // Not valid age ciphertext, so decryption fails.
        let bundle_dir = profile.encrypted_backup_path(&ctx);
        fs::create_dir_all(&bundle_dir).unwrap();
        fs::write(
            bundle_dir.join(ENCRYPTED_BUNDLE_FILE),
            b"not age ciphertext, just garbage bytes",
        )
        .unwrap();

        let report = restore_encrypted_configs(&ctx, &profile, &password);

        assert!(report.outcomes.is_empty());
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("could not decrypt"));
    }

    #[test]
    fn bundle_with_unreadable_tar_produces_warning_not_hard_error() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let profile = ActiveProfile::common_only();
        let password = SecretString::from("pw".to_string());

        let mut entries = HashMap::new();
        entries.insert(
            "secret".to_string(),
            entry("secret.txt", dir.path().join("target.txt")),
        );
        save_registry(&ctx, entries);

        // Decrypts fine, but the payload isn't a valid tar archive.
        let not_a_tar = dir.path().join("not-a-tar");
        fs::write(&not_a_tar, b"not a tar file at all, just garbage bytes").unwrap();
        let bundle_dir = profile.encrypted_backup_path(&ctx);
        fs::create_dir_all(&bundle_dir).unwrap();
        encrypt_file(
            &not_a_tar,
            &bundle_dir.join(ENCRYPTED_BUNDLE_FILE),
            &password,
        )
        .unwrap();

        let report = restore_encrypted_configs(&ctx, &profile, &password);

        assert!(report.outcomes.is_empty());
        assert_eq!(report.warnings.len(), 1);
        assert!(
            report.warnings[0].contains("could not read encrypted bundle archive"),
            "unexpected warning: {}",
            report.warnings[0]
        );
    }

    #[test]
    fn restore_file_creates_missing_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested/deep/target.txt");

        let note = restore_file(&target, b"nested content").unwrap();

        assert!(note.is_none());
        assert_eq!(fs::read(&target).unwrap(), b"nested content");
    }
}
