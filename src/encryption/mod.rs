//! Age-passphrase file encryption, tar bundling for encrypted backups, and
//! the system-keychain-backed password store.

/// Tar bundling and temp-file helpers used by encrypted backup/restore.
mod bundle;
/// System-keychain-backed storage for the encryption password.
pub mod keyring;

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use age::secrecy::SecretString;
pub use bundle::{
    create_temp_file, load_tar_member_map, set_private_file_permissions, write_entries_tar,
};

use crate::error::{Result, WrapErr};

/// Encrypt `source` into `dest` with an age passphrase.
pub fn encrypt_file(source: &Path, dest: &Path, password: &SecretString) -> Result<()> {
    let content = fs::read(source)
        .wrap_err_with(|| format!("Read source file for encryption: {}", source.display()))?;

    let encryptor = age::Encryptor::with_user_passphrase(password.clone());

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("Create parent directory: {}", parent.display()))?;
    }

    let mut encrypted = vec![];
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .wrap_err("Initialize encryptor")?;
    writer
        .write_all(&content)
        .wrap_err("Write encrypted content")?;
    writer.finish().wrap_err("Finalize encryption output")?;

    fs::write(dest, encrypted)
        .wrap_err_with(|| format!("Write encrypted file: {}", dest.display()))?;
    Ok(())
}

/// Decrypt `source` into `dest`. On unix the restored file is made private
/// (mode 0600).
pub fn decrypt_file(source: &Path, dest: &Path, password: &SecretString) -> Result<()> {
    let encrypted = fs::read(source)
        .wrap_err_with(|| format!("Read encrypted file for decryption: {}", source.display()))?;

    let decryptor = age::Decryptor::new(&encrypted[..]).wrap_err("Create decryptor")?;
    let identity = age::scrypt::Identity::new(password.clone());

    let mut decrypted = vec![];
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .wrap_err("Decrypt payload")?;
    reader
        .read_to_end(&mut decrypted)
        .wrap_err("Read decrypted payload")?;

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("Create parent directory: {}", parent.display()))?;
    }

    fs::write(dest, decrypted)
        .wrap_err_with(|| format!("Write decrypted file: {}", dest.display()))?;

    set_private_file_permissions(dest)?;

    Ok(())
}

/// Path of the per-file encrypted backup for a source path.
pub fn get_encrypted_path(source_path: &str) -> String {
    format!("{}.age", source_path)
}
