use std::path::{Path, PathBuf};

use directories_next::BaseDirs;

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
        let base_dirs = BaseDirs::new().ok_or(Error::NoHomeDir)?;
        Ok(Self {
            root: base_dirs.home_dir().join(".dotfm"),
        })
    }

    /// Use a custom root directory.
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn backup_dir(&self) -> PathBuf {
        self.root.join(BACKUP_DIR)
    }

    pub fn common_dir(&self) -> PathBuf {
        self.backup_dir().join(COMMON_DIR)
    }

    pub fn encrypted_common_dir(&self) -> PathBuf {
        self.common_dir().join(ENCRYPTED_DIR)
    }

    pub fn profile_dir(&self, profile_name: &str) -> PathBuf {
        self.backup_dir().join(PROFILES_DIR).join(profile_name)
    }

    pub fn encrypted_profile_dir(&self, profile_name: &str) -> PathBuf {
        self.profile_dir(profile_name).join(ENCRYPTED_DIR)
    }

    pub fn packages_dir(&self) -> PathBuf {
        self.backup_dir().join(PACKAGES_DIR)
    }

    pub fn config_registry_path(&self) -> PathBuf {
        self.root.join("config.registry.json")
    }

    pub fn package_registry_path(&self) -> PathBuf {
        self.root.join("package.registry.json")
    }

    pub fn encrypted_registry_path(&self) -> PathBuf {
        self.root.join("encrypted.registry.json")
    }

    pub fn profiles_config_path(&self) -> PathBuf {
        self.root.join(PROFILE_CONFIG_FILE)
    }

    pub fn active_profile_path(&self) -> PathBuf {
        self.root.join(ACTIVE_PROFILE_FILE)
    }
}

pub(crate) fn get_xdg_or_default_config_path(relative_path: &str) -> PathBuf {
    if let Some(xdg_config) = xdg_config_home_dir() {
        return xdg_config.join(relative_path);
    }
    BaseDirs::new()
        .unwrap()
        .home_dir()
        .join(".config")
        .join(relative_path)
}

fn xdg_config_home_dir() -> Option<PathBuf> {
    match std::env::var_os("XDG_CONFIG_HOME") {
        Some(value) if !value.is_empty() => Some(PathBuf::from(value)),
        _ => None,
    }
}
