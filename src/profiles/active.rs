use std::fs;
use std::io;
use std::path::PathBuf;

use crate::context::Dotfm;

/// The profile a backup or restore run targets: either a named profile
/// layered on top of `common`, or `common` alone.
#[derive(Debug, Clone)]
pub struct ActiveProfile {
    pub name: Option<String>,
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
    /// Target the named profile.
    pub fn with_profile(name: &str) -> Self {
        Self {
            name: Some(name.to_string()),
        }
    }

    /// Target the `common` layer only.
    pub fn common_only() -> Self {
        Self { name: None }
    }

    /// Resolve the target profile: an explicit override wins, then the
    /// `DOTFM_PROFILE` environment variable, then the persisted active
    /// profile, then `common`.
    pub fn resolve(ctx: &Dotfm, override_profile: Option<&str>) -> Self {
        if let Some(profile) = override_profile {
            return Self::with_profile(profile);
        }

        if let Some(profile) = get_active_profile_name(ctx) {
            return Self::with_profile(&profile);
        }

        Self::common_only()
    }

    /// This profile's backup directory (profile layer or `common`).
    pub fn backup_path(&self, ctx: &Dotfm) -> PathBuf {
        match &self.name {
            Some(name) => ctx.profile_dir(name),
            None => ctx.common_dir(),
        }
    }

    /// This profile's encrypted-files directory (profile layer or `common`).
    pub fn encrypted_backup_path(&self, ctx: &Dotfm) -> PathBuf {
        match &self.name {
            Some(name) => ctx.encrypted_profile_dir(name),
            None => ctx.encrypted_common_dir(),
        }
    }
}

/// Name of the active profile, from `DOTFM_PROFILE` or the persisted
/// `.active-profile` file.
pub fn get_active_profile_name(ctx: &Dotfm) -> Option<String> {
    if let Ok(profile) = std::env::var("DOTFM_PROFILE") {
        let trimmed = profile.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let active_profile_path = ctx.active_profile_path();
    if let Ok(profile) = fs::read_to_string(&active_profile_path) {
        let trimmed = profile.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

/// Persist `profile_name` as the active profile.
pub fn set_active_profile(ctx: &Dotfm, profile_name: &str) -> io::Result<()> {
    let active_profile_path = ctx.active_profile_path();
    if let Some(parent) = active_profile_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(active_profile_path, profile_name)
}

/// Remove the persisted active profile, if any.
pub fn clear_active_profile(ctx: &Dotfm) -> io::Result<()> {
    let active_profile_path = ctx.active_profile_path();
    if active_profile_path.exists() {
        fs::remove_file(active_profile_path)?;
    }
    Ok(())
}
