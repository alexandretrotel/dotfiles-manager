use std::fs;
use std::io;
use std::path::PathBuf;

use crate::utils::paths::get_encrypted_common_path;
use crate::utils::paths::get_encrypted_profiles_path;
use crate::utils::paths::{get_active_profile_path, get_common_path, get_profiles_path};

#[derive(Debug, Clone)]
pub(crate) struct ActiveProfile {
    pub(crate) name: Option<String>,
}

impl std::fmt::Display for ActiveProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(name) => write!(f, "profile={}", name),
            None => write!(f, "common (no active profile)"),
        }
    }
}

impl ActiveProfile {
    pub(crate) fn with_profile(name: &str) -> Self {
        Self {
            name: Some(name.to_string()),
        }
    }

    pub(crate) fn common_only() -> Self {
        Self { name: None }
    }

    pub(crate) fn resolve(cli_profile: Option<&str>) -> Self {
        if let Some(profile) = cli_profile {
            return Self::with_profile(profile);
        }

        if let Some(profile) = get_active_profile_name() {
            return Self::with_profile(&profile);
        }

        Self::common_only()
    }

    pub(crate) fn get_backup_path(&self) -> PathBuf {
        match &self.name {
            Some(name) => get_profiles_path(name),
            None => get_common_path(),
        }
    }

    pub(crate) fn get_encrypted_backup_path(&self) -> PathBuf {
        match &self.name {
            Some(name) => get_encrypted_profiles_path(name),
            None => get_encrypted_common_path(),
        }
    }
}

pub(crate) fn get_active_profile_name() -> Option<String> {
    if let Ok(profile) = std::env::var("DOTFM_PROFILE") {
        let trimmed = profile.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let active_profile_path = get_active_profile_path();
    if let Ok(profile) = fs::read_to_string(&active_profile_path) {
        let trimmed = profile.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

pub(crate) fn set_active_profile(profile_name: &str) -> io::Result<()> {
    let active_profile_path = get_active_profile_path();
    if let Some(parent) = active_profile_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(active_profile_path, profile_name)
}

pub(crate) fn clear_active_profile() -> io::Result<()> {
    let active_profile_path = get_active_profile_path();
    if active_profile_path.exists() {
        fs::remove_file(active_profile_path)?;
    }
    Ok(())
}
