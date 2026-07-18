mod bundle;

use age::secrecy::ExposeSecret;
use age::secrecy::SecretString;
use eyre::{Result, WrapErr, bail};
use keyring_core::{Entry, Error as KeyringError, set_default_store};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::OnceLock;

const KEYRING_SERVICE: &str = "mntn";
const KEYRING_USERNAME: &str = "encryption";

#[cfg(target_os = "macos")]
fn init_default_keyring_store() -> Result<()> {
    set_default_store(apple_native_keyring_store::keychain::Store::new()?);
    Ok(())
}

#[cfg(target_os = "windows")]
fn init_default_keyring_store() -> Result<()> {
    set_default_store(windows_native_keyring_store::Store::new()?);
    Ok(())
}

#[cfg(target_os = "linux")]
fn init_default_keyring_store() -> Result<()> {
    set_default_store(zbus_secret_service_keyring_store::Store::new()?);
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn init_default_keyring_store() -> Result<()> {
    bail!("No supported keyring store configured for this operating system");
}

fn keyring_entry() -> eyre::Result<Entry> {
    static INIT: OnceLock<Result<()>> = OnceLock::new();
    INIT.get_or_init(init_default_keyring_store)
        .as_ref()
        .map_err(|e| eyre::eyre!(e.to_string()))?;
    Entry::new(KEYRING_SERVICE, KEYRING_USERNAME).map_err(eyre::Report::from)
}

pub(crate) use bundle::{
    create_temp_path, load_tar_member_map, set_private_file_permissions, write_entries_tar,
};

pub(crate) fn prompt_password(confirm: bool) -> Result<SecretString> {
    let password =
        rpassword::prompt_password("Enter encryption password: ").wrap_err("Read password")?;

    if password.is_empty() {
        bail!("Password cannot be empty");
    }

    if confirm {
        let confirmation = rpassword::prompt_password("Confirm encryption password: ")
            .wrap_err("Read password confirmation")?;
        if password != confirmation {
            bail!("Passwords do not match");
        }
    }

    Ok(SecretString::new(password.into()))
}

fn read_stored_password() -> Option<SecretString> {
    let entry = keyring_entry().ok()?;
    let password = entry.get_password().ok()?;
    (!password.is_empty()).then_some(SecretString::new(password.into()))
}

pub(crate) fn persist_encryption_password() -> Result<()> {
    let password =
        prompt_password(true).wrap_err("Read encryption password for system keychain")?;
    let entry = keyring_entry().wrap_err("Open system keychain")?;
    entry
        .set_password(password.expose_secret())
        .wrap_err("Save encryption password to system keychain")?;
    Ok(())
}

pub(crate) fn clear_stored_encryption_password() -> Result<()> {
    let entry = keyring_entry().wrap_err("Open system keychain")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => {
            Err(eyre::Report::from(e)).wrap_err("Remove encryption password from system keychain")
        }
    }
}

pub(crate) fn resolve_encryption_password(
    ask_password: bool,
    confirm_on_prompt: bool,
) -> Result<SecretString> {
    let stored = read_stored_password();
    let had_stored = stored.is_some();
    if !ask_password && let Some(password) = stored {
        return Ok(password);
    }
    let password = prompt_password(confirm_on_prompt)?;
    if !had_stored {
        eprintln!(
            "Tip: run `mntn secret set` to save this password in your system keychain and skip prompts later."
        );
    }
    Ok(password)
}

pub(crate) fn encrypt_file(source: &Path, dest: &Path, password: &SecretString) -> Result<()> {
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

pub(crate) fn decrypt_file(source: &Path, dest: &Path, password: &SecretString) -> Result<()> {
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

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        fs::set_permissions(dest, permissions)
            .wrap_err_with(|| format!("Set permissions on: {}", dest.display()))?;
    }

    Ok(())
}

pub(crate) fn get_encrypted_path(source_path: &str) -> String {
    format!("{}.age", source_path)
}
