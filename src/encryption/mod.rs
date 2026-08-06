//! Age-passphrase file encryption, tar bundling for encrypted backups, and
//! the system-keychain-backed password store.

mod bundle;
pub mod keyring;

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use age::secrecy::SecretString;
pub use bundle::{
    TarEntryRef, TarMemberMap, TarSourceEntry, create_temp_file, enumerate_tar_files,
    load_tar_files, set_private_file_permissions, write_tar_archive,
};

use crate::error::{Result, WrapErr};

/// Encrypt `source` into `destination` with an age passphrase.
pub fn encrypt_file(source: &Path, destination: &Path, password: &SecretString) -> Result<()> {
    let content = fs::read(source)
        .wrap_err_with(|| format!("Read source file for encryption: {}", source.display()))?;

    let encryptor = age::Encryptor::with_user_passphrase(password.clone());

    if let Some(parent) = destination.parent() {
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

    fs::write(destination, encrypted)
        .wrap_err_with(|| format!("Write encrypted file: {}", destination.display()))?;
    Ok(())
}

/// Decrypt `source` into `destination`. On unix the restored file is made private
/// (mode 0600).
pub fn decrypt_file(source: &Path, destination: &Path, password: &SecretString) -> Result<()> {
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

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("Create parent directory: {}", parent.display()))?;
    }

    fs::write(destination, decrypted)
        .wrap_err_with(|| format!("Write decrypted file: {}", destination.display()))?;

    set_private_file_permissions(destination)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_then_decrypt_round_trips_content() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("secret.txt");
        let encrypted = dir.path().join("secret.age");
        let decrypted = dir.path().join("secret.out");

        fs::write(&source, b"hello, world").unwrap();
        let password = SecretString::from("correct horse battery staple".to_string());

        encrypt_file(&source, &encrypted, &password).unwrap();
        assert!(encrypted.exists());
        assert_ne!(fs::read(&encrypted).unwrap(), fs::read(&source).unwrap());

        decrypt_file(&encrypted, &decrypted, &password).unwrap();
        assert_eq!(fs::read(&decrypted).unwrap(), b"hello, world");
    }

    #[test]
    fn decrypt_with_wrong_password_is_password_error() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("secret.txt");
        let encrypted = dir.path().join("secret.age");
        let decrypted = dir.path().join("secret.out");

        fs::write(&source, b"top secret").unwrap();
        let password = SecretString::from("right password".to_string());
        let wrong_password = SecretString::from("wrong password".to_string());

        encrypt_file(&source, &encrypted, &password).unwrap();

        let err = decrypt_file(&encrypted, &decrypted, &wrong_password).unwrap_err();
        assert!(err.is_password_error());
    }

    #[test]
    fn encrypt_creates_missing_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("secret.txt");
        let encrypted = dir.path().join("nested/deep/secret.age");

        fs::write(&source, b"nested content").unwrap();
        let password = SecretString::from("password".to_string());

        encrypt_file(&source, &encrypted, &password).unwrap();
        assert!(encrypted.exists());
    }

    #[cfg(unix)]
    #[test]
    fn decrypt_sets_private_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("secret.txt");
        let encrypted = dir.path().join("secret.age");
        let decrypted = dir.path().join("secret.out");

        fs::write(&source, b"permissions matter").unwrap();
        let password = SecretString::from("password".to_string());

        encrypt_file(&source, &encrypted, &password).unwrap();
        decrypt_file(&encrypted, &decrypted, &password).unwrap();

        let mode = fs::metadata(&decrypted).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
