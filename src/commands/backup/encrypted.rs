use crate::encryption::{
    create_temp_path, encrypt_file, resolve_encryption_password, write_entries_tar,
};
use crate::registry::encrypted::EncryptedRegistry;
use crate::utils::{
    display::{green, yellow},
    paths::{ENCRYPTED_BUNDLE_FILE, get_encrypted_registry_path},
};
use age::secrecy::SecretString;
use color_eyre::eyre::{Result, WrapErr};
use std::fs;
use std::path::Path;

pub fn backup_encrypted_configs(
    encrypted_backup_path: &Path,
    ask_password: bool,
) -> Result<(u32, u32)> {
    let password = resolve_encryption_password(ask_password, true)
        .wrap_err("Prompt for encryption password before encrypted backup")?;

    backup_encrypted_configs_with_password(encrypted_backup_path, &password)
}

fn backup_encrypted_configs_with_password(
    encrypted_backup_path: &Path,
    password: &SecretString,
) -> Result<(u32, u32)> {
    let registry_path = get_encrypted_registry_path();
    let encrypted_registry = EncryptedRegistry::load_or_create(&registry_path)
        .wrap_err_with(|| format!("Load encrypted registry: {}", registry_path.display()))?;

    let enabled_entries: Vec<_> = encrypted_registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        println!("No encrypted configuration files found to backup");
        return Ok((0, 0));
    }

    println!("   Encrypted configs: {} entries", enabled_entries.len());

    let mut skipped: u32 = 0;
    let mut to_archive: Vec<(String, std::path::PathBuf)> = Vec::new();

    for (id, entry) in enabled_entries {
        if !entry.target_path.exists() {
            skipped += 1;
            println!(
                "{}",
                yellow(&format!(
                    "     skipped missing target {} ({})",
                    entry.target_path.display(),
                    id
                ))
            );
            continue;
        }

        if entry.target_path.is_dir() {
            skipped += 1;
            println!(
                "{}",
                yellow(&format!(
                    "     skipped directory {} ({})",
                    entry.target_path.display(),
                    id
                ))
            );
            continue;
        }

        to_archive.push((entry.source_path.clone(), entry.target_path.clone()));
    }

    to_archive.sort_by(|a, b| a.0.cmp(&b.0));

    let bundle_destination = encrypted_backup_path.join(ENCRYPTED_BUNDLE_FILE);

    if to_archive.is_empty() {
        let _ = fs::remove_file(&bundle_destination);
        return Ok((0, skipped));
    }

    let tar_refs: Vec<(&str, &Path)> = to_archive
        .iter()
        .map(|(source, target)| (source.as_str(), target.as_path()))
        .collect();

    let tar_temp = create_temp_path("enc-bundle-tar").wrap_err("Create temporary tar path")?;

    let backup_result: Result<()> = (|| {
        write_entries_tar(&tar_temp, &tar_refs)?;
        encrypt_file(&tar_temp, &bundle_destination, password).wrap_err("Encrypt config bundle")?;
        Ok(())
    })();

    let _ = fs::remove_file(&tar_temp);

    backup_result?;

    let succeeded = to_archive.len();
    for (source_path, _) in &to_archive {
        println!("     {} {}", green("✔"), source_path);
    }

    Ok((succeeded as u32, skipped))
}
