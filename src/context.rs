use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub(crate) const BACKUP_DIR: &str = "backup";
pub(crate) const COMMON_DIR: &str = "common";
pub(crate) const ENCRYPTED_DIR: &str = "encrypted";
pub(crate) const PROFILES_DIR: &str = "profiles";
pub(crate) const PACKAGES_DIR: &str = "packages";

/// Name of the encrypted bundle file inside an encrypted backup directory.
pub const ENCRYPTED_BUNDLE_FILE: &str = "dotfm-encrypted-bundle.age";
/// Name of the profile configuration file inside the dotfm root.
pub const PROFILE_CONFIG_FILE: &str = "profiles.json";
pub(crate) const ACTIVE_PROFILE_FILE: &str = ".active-profile";

/// Handle to a dotfm data directory (by default `~/.dotfm`).
///
/// All library operations take a `&Dotfm` so tests and embedders can point
/// them at any root directory.
#[derive(Debug, Clone)]
pub struct Dotfm {
    root: PathBuf,
}

impl Dotfm {
    /// Open the default root at `~/.dotfm`.
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().ok_or(Error::NoHomeDir)?;
        Ok(Self {
            root: home_dir.join(".dotfm"),
        })
    }

    /// Use a custom root directory.
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The dotfm data directory itself.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Directory holding all backup output.
    pub fn backup_dir(&self) -> PathBuf {
        self.root.join(BACKUP_DIR)
    }

    /// Directory holding the profile-independent (`common`) backup layer.
    pub fn common_dir(&self) -> PathBuf {
        self.backup_dir().join(COMMON_DIR)
    }

    /// Directory holding the `common` layer's encrypted files.
    pub fn encrypted_common_dir(&self) -> PathBuf {
        self.common_dir().join(ENCRYPTED_DIR)
    }

    /// Directory holding `profile_name`'s backup layer.
    pub fn profile_dir(&self, profile_name: &str) -> PathBuf {
        self.backup_dir().join(PROFILES_DIR).join(profile_name)
    }

    /// Directory holding `profile_name`'s encrypted files.
    pub fn encrypted_profile_dir(&self, profile_name: &str) -> PathBuf {
        self.profile_dir(profile_name).join(ENCRYPTED_DIR)
    }

    /// Directory holding package-manager export files.
    pub fn packages_dir(&self) -> PathBuf {
        self.backup_dir().join(PACKAGES_DIR)
    }

    /// Path to the config registry file.
    pub fn config_registry_path(&self) -> PathBuf {
        self.root.join("config.registry.json")
    }

    /// Path to the package registry file.
    pub fn package_registry_path(&self) -> PathBuf {
        self.root.join("package.registry.json")
    }

    /// Path to the encrypted-configs registry file.
    pub fn encrypted_registry_path(&self) -> PathBuf {
        self.root.join("encrypted.registry.json")
    }

    /// Path to the profile configuration file.
    pub fn profiles_config_path(&self) -> PathBuf {
        self.root.join(PROFILE_CONFIG_FILE)
    }

    /// Path to the file recording the active profile.
    pub fn active_profile_path(&self) -> PathBuf {
        self.root.join(ACTIVE_PROFILE_FILE)
    }
}
