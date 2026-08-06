use std::sync::OnceLock;

use age::secrecy::{ExposeSecret, SecretString};
use keyring_core::{Entry, Error as KeyringError, set_default_store};

use crate::error::{Error, Result, WrapErr};

const KEYRING_SERVICE: &str = "dotfiles-manager";
const KEYRING_USERNAME: &str = "encryption";

/// Register the platform-native keychain backend as the default
/// `keyring-core` store (macOS Keychain / Windows Credential Manager /
/// Linux Secret Service).
#[cfg(target_os = "macos")]
fn init_default_keyring_store() -> Result<()> {
    set_default_store(apple_native_keyring_store::keychain::Store::new()?);
    Ok(())
}

/// Register the platform-native keychain backend as the default
/// `keyring-core` store (macOS Keychain / Windows Credential Manager /
/// Linux Secret Service).
#[cfg(target_os = "windows")]
fn init_default_keyring_store() -> Result<()> {
    set_default_store(windows_native_keyring_store::Store::new()?);
    Ok(())
}

/// Register the platform-native keychain backend as the default
/// `keyring-core` store (macOS Keychain / Windows Credential Manager /
/// Linux Secret Service).
#[cfg(target_os = "linux")]
fn init_default_keyring_store() -> Result<()> {
    set_default_store(zbus_secret_service_keyring_store::Store::new()?);
    Ok(())
}

/// No native keychain store is available on this platform; keyring
/// operations always fail.
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn init_default_keyring_store() -> Result<()> {
    Err(Error::Keyring(
        "No supported keyring store configured for this operating system".to_string(),
    ))
}

/// The dfm entry in the system keychain, initializing the default store
/// on first call.
fn keyring_entry() -> Result<Entry> {
    static INIT: OnceLock<Result<()>> = OnceLock::new();
    INIT.get_or_init(init_default_keyring_store)
        .as_ref()
        .map_err(|e| Error::Keyring(e.to_string()))?;
    Entry::new(KEYRING_SERVICE, KEYRING_USERNAME).map_err(Error::from)
}

/// The encryption password stored in the system keychain, if any.
pub fn get_stored_password() -> Option<SecretString> {
    let entry = keyring_entry().ok()?;
    let password = entry.get_password().ok()?;
    (!password.is_empty()).then_some(SecretString::new(password.into()))
}

/// Store the encryption password in the system keychain.
pub fn set_stored_password(password: &SecretString) -> Result<()> {
    let entry = keyring_entry().wrap_err("Open system keychain")?;
    entry
        .set_password(password.expose_secret())
        .wrap_err("Save encryption password to system keychain")?;
    Ok(())
}

/// Remove the encryption password from the system keychain. Succeeds when no
/// password was stored.
pub fn clear_stored_password() -> Result<()> {
    let entry = keyring_entry().wrap_err("Open system keychain")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(Error::from(e)).wrap_err("Remove encryption password from system keychain"),
    }
}
