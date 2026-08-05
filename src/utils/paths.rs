use directories_next::BaseDirs;
use std::path::PathBuf;

pub(crate) const BACKUP_DIR: &str = "backup";
pub(crate) const COMMON_DIR: &str = "common";
pub(crate) const ENCRYPTED_DIR: &str = "encrypted";
pub(crate) const ENCRYPTED_BUNDLE_FILE: &str = "dotfm-encrypted-bundle.age";

pub(crate) const PROFILES_DIR: &str = "profiles";
pub(crate) const PROFILE_CONFIG_FILE: &str = "profiles.json";
pub(crate) const ACTIVE_PROFILE_FILE: &str = ".active-profile";

pub(crate) fn get_dotfm_dir() -> PathBuf {
    let base_dirs = BaseDirs::new().unwrap();
    let home_dir = base_dirs.home_dir();
    home_dir.join(".dotfm")
}

pub(crate) fn get_backup_path() -> PathBuf {
    get_dotfm_dir().join(BACKUP_DIR)
}

pub(crate) fn get_common_path() -> PathBuf {
    get_backup_path().join(COMMON_DIR)
}

pub(crate) fn get_encrypted_common_path() -> PathBuf {
    get_common_path().join(ENCRYPTED_DIR)
}

pub(crate) fn get_profiles_path(profile_name: &str) -> PathBuf {
    get_backup_path().join(PROFILES_DIR).join(profile_name)
}

pub(crate) fn get_encrypted_profiles_path(profile_name: &str) -> PathBuf {
    get_profiles_path(profile_name).join(ENCRYPTED_DIR)
}

pub(crate) fn get_config_registry_path() -> PathBuf {
    get_dotfm_dir().join("config.registry.json")
}

pub(crate) fn get_package_registry_path() -> PathBuf {
    get_dotfm_dir().join("package.registry.json")
}

pub(crate) fn get_encrypted_registry_path() -> PathBuf {
    get_dotfm_dir().join("encrypted.registry.json")
}

pub(crate) fn get_packages_path() -> PathBuf {
    get_backup_path().join("packages")
}

pub(crate) fn get_profiles_config_path() -> PathBuf {
    get_dotfm_dir().join(PROFILE_CONFIG_FILE)
}

pub(crate) fn get_active_profile_path() -> PathBuf {
    get_dotfm_dir().join(ACTIVE_PROFILE_FILE)
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

pub(crate) fn get_ghostty_config_path() -> PathBuf {
    if xdg_config_home_dir().is_some() {
        return get_xdg_or_default_config_path("ghostty/config");
    }

    #[cfg(target_os = "macos")]
    {
        BaseDirs::new()
            .unwrap()
            .home_dir()
            .join("Library/Application Support/com.mitchellh.ghostty/config")
    }

    #[cfg(not(target_os = "macos"))]
    {
        get_xdg_or_default_config_path("ghostty/config")
    }
}

fn xdg_config_home_dir() -> Option<PathBuf> {
    match std::env::var_os("XDG_CONFIG_HOME") {
        Some(value) if !value.is_empty() => Some(PathBuf::from(value)),
        _ => None,
    }
}
