use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub(crate) const BACKUP_DIR: &str = "backup";
pub(crate) const COMMON_DIR: &str = "common";
pub(crate) const ENCRYPTED_DIR: &str = "encrypted";
pub(crate) const PROFILES_DIR: &str = "profiles";
pub(crate) const PACKAGES_DIR: &str = "packages";

/// Name of the encrypted bundle file inside an encrypted backup directory.
pub const ENCRYPTED_BUNDLE_FILE: &str = "dfm-encrypted-bundle.age";
/// Name of the profile configuration file inside the dfm root.
pub const PROFILE_CONFIG_FILE: &str = "profiles.json";
pub(crate) const ACTIVE_PROFILE_FILE: &str = ".active-profile";

/// Handle to a dfm data directory (by default `~/.dfm`).
///
/// All library operations take a `&Dfm` so tests and embedders can point
/// them at any root directory.
#[derive(Debug, Clone)]
pub struct Dfm {
    root: PathBuf,
}

impl Dfm {
    /// Open the default root at `~/.dfm`.
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().ok_or(Error::NoHomeDir)?;
        Ok(Self {
            root: home_dir.join(".dfm"),
        })
    }

    /// Use a custom root directory.
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The dfm data directory itself.
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

    /// Directory holding all named profiles' backup layers.
    pub fn profiles_root_dir(&self) -> PathBuf {
        self.backup_dir().join(PROFILES_DIR)
    }

    /// Directory holding `profile_name`'s backup layer.
    pub fn profile_dir(&self, profile_name: &str) -> PathBuf {
        self.profiles_root_dir().join(profile_name)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_dfm() -> Dfm {
        Dfm::with_root(PathBuf::from("/tmp/fake-dfm-root"))
    }

    #[test]
    fn with_root_stores_the_given_root() {
        assert_eq!(fake_dfm().root(), Path::new("/tmp/fake-dfm-root"));
    }

    #[test]
    fn backup_dir_is_under_root() {
        assert_eq!(
            fake_dfm().backup_dir(),
            PathBuf::from("/tmp/fake-dfm-root/backup")
        );
    }

    #[test]
    fn common_dir_is_under_backup_dir() {
        assert_eq!(
            fake_dfm().common_dir(),
            PathBuf::from("/tmp/fake-dfm-root/backup/common")
        );
    }

    #[test]
    fn encrypted_common_dir_is_under_common_dir() {
        assert_eq!(
            fake_dfm().encrypted_common_dir(),
            PathBuf::from("/tmp/fake-dfm-root/backup/common/encrypted")
        );
    }

    #[test]
    fn profiles_root_dir_is_under_backup_dir() {
        assert_eq!(
            fake_dfm().profiles_root_dir(),
            PathBuf::from("/tmp/fake-dfm-root/backup/profiles")
        );
    }

    #[test]
    fn profile_dir_is_under_profiles_dir() {
        assert_eq!(
            fake_dfm().profile_dir("work"),
            PathBuf::from("/tmp/fake-dfm-root/backup/profiles/work")
        );
    }

    #[test]
    fn encrypted_profile_dir_is_under_profile_dir() {
        assert_eq!(
            fake_dfm().encrypted_profile_dir("work"),
            PathBuf::from("/tmp/fake-dfm-root/backup/profiles/work/encrypted")
        );
    }

    #[test]
    fn packages_dir_is_under_backup_dir() {
        assert_eq!(
            fake_dfm().packages_dir(),
            PathBuf::from("/tmp/fake-dfm-root/backup/packages")
        );
    }

    #[test]
    fn config_registry_path_is_under_root() {
        assert_eq!(
            fake_dfm().config_registry_path(),
            PathBuf::from("/tmp/fake-dfm-root/config.registry.json")
        );
    }

    #[test]
    fn package_registry_path_is_under_root() {
        assert_eq!(
            fake_dfm().package_registry_path(),
            PathBuf::from("/tmp/fake-dfm-root/package.registry.json")
        );
    }

    #[test]
    fn encrypted_registry_path_is_under_root() {
        assert_eq!(
            fake_dfm().encrypted_registry_path(),
            PathBuf::from("/tmp/fake-dfm-root/encrypted.registry.json")
        );
    }

    #[test]
    fn profiles_config_path_is_under_root() {
        assert_eq!(
            fake_dfm().profiles_config_path(),
            PathBuf::from("/tmp/fake-dfm-root/profiles.json")
        );
    }

    #[test]
    fn active_profile_path_is_under_root() {
        assert_eq!(
            fake_dfm().active_profile_path(),
            PathBuf::from("/tmp/fake-dfm-root/.active-profile")
        );
    }
}
