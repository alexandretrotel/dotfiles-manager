use std::fs;
use std::path::PathBuf;

use super::{ProfileConfig, clear_active_profile, get_active_profile_name, set_active_profile};
use crate::context::Dotfm;
use crate::error::{Error, Result, WrapErr};

/// Profile names that mean "no profile, common layer only".
pub const COMMON_PROFILE_NAMES: [&str; 2] = ["common", "none"];

/// A profile that was just created by [`create_profile`].
#[derive(Debug, Clone)]
pub struct CreatedProfile {
    pub name: String,
    pub description: Option<String>,
    /// Directory created for the profile's backups.
    pub directory: PathBuf,
}

/// A profile that was just removed from the profile config by
/// [`delete_profile`].
#[derive(Debug, Clone)]
pub struct DeletedProfile {
    pub name: String,
    /// Backup directory left on disk (never removed automatically), if any.
    pub retained_directory: Option<PathBuf>,
}

/// Result of switching profiles.
#[derive(Debug, Clone)]
pub enum SwitchProfileOutcome {
    /// Active profile was cleared; runs now use the common layer only.
    Cleared,
    /// The requested profile was already active.
    AlreadyActive(String),
    /// Switched to the named profile.
    Switched(String),
}

/// Register a new profile and create its backup directory. Fails if the
/// name is empty, invalid, or already taken.
pub fn create_profile(
    ctx: &Dotfm,
    name: &str,
    description: Option<String>,
) -> Result<CreatedProfile> {
    let path = ctx.profiles_config_path();
    let mut config = ProfileConfig::load_or_default(ctx);

    if config.profile_exists(name) {
        return Err(Error::ProfileExists(name.to_string()));
    }

    if name.is_empty() {
        return Err(Error::EmptyProfileName);
    }

    if name
        .chars()
        .any(|c| !c.is_alphanumeric() && c != '-' && c != '_')
    {
        return Err(Error::InvalidProfileName(name.to_string()));
    }

    config.create_profile(name, description.clone());
    if config.version.is_empty() {
        config.version = "1.0.0".to_string();
    }

    config
        .save(&path)
        .wrap_err_with(|| format!("Save profile config to {}", path.display()))?;

    let profile_dir = ctx.profile_dir(name);
    fs::create_dir_all(&profile_dir).wrap_err_with(|| {
        format!(
            "Create profile directory at {} (config was saved)",
            profile_dir.display()
        )
    })?;

    Ok(CreatedProfile {
        name: name.to_string(),
        description,
        directory: profile_dir,
    })
}

/// Remove a profile from the profile config. Fails if it doesn't exist or is
/// currently active; its backup directory (if any) is left on disk.
pub fn delete_profile(ctx: &Dotfm, name: &str) -> Result<DeletedProfile> {
    let path = ctx.profiles_config_path();
    let mut config = ProfileConfig::load_or_default(ctx);

    if !config.profile_exists(name) {
        return Err(Error::ProfileNotFound(name.to_string()));
    }

    if let Some(current) = get_active_profile_name(ctx)
        && current == name
    {
        return Err(Error::DeleteActiveProfile(name.to_string()));
    }

    config.delete_profile(name);
    config
        .save(&path)
        .wrap_err_with(|| format!("Save profile config to {}", path.display()))?;

    let profile_dir = ctx.profile_dir(name);
    let retained_directory = profile_dir.exists().then_some(profile_dir);

    Ok(DeletedProfile {
        name: name.to_string(),
        retained_directory,
    })
}

/// Switch the active profile. Passing `common` or `none` clears it.
pub fn switch_profile(ctx: &Dotfm, name: &str) -> Result<SwitchProfileOutcome> {
    if COMMON_PROFILE_NAMES.contains(&name) {
        clear_active_profile(ctx)?;
        return Ok(SwitchProfileOutcome::Cleared);
    }

    let config = ProfileConfig::load_or_default(ctx);
    if !config.profile_exists(name) {
        return Err(Error::ProfileNotFound(name.to_string()));
    }

    if get_active_profile_name(ctx).as_deref() == Some(name) {
        return Ok(SwitchProfileOutcome::AlreadyActive(name.to_string()));
    }

    set_active_profile(ctx, name)?;
    Ok(SwitchProfileOutcome::Switched(name.to_string()))
}
