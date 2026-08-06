use std::fs;
use std::path::PathBuf;

use super::{ProfileConfig, clear_active_profile, get_active_profile_name, set_active_profile};
use crate::context::Dfm;
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
    ctx: &Dfm,
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
pub fn delete_profile(ctx: &Dfm, name: &str) -> Result<DeletedProfile> {
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
pub fn switch_profile(ctx: &Dfm, name: &str) -> Result<SwitchProfileOutcome> {
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

#[cfg(test)]
mod tests {
    use std::sync::MutexGuard;

    use super::*;
    use crate::profiles::active::ACTIVE_PROFILE_ENV_LOCK;

    // Holds `ACTIVE_PROFILE_ENV_LOCK` for the caller's whole test body, since
    // `create_profile`/`delete_profile`/`switch_profile` read `DFM_PROFILE`
    // transitively and must not race the env-var tests in `active.rs`.
    fn ctx() -> (tempfile::TempDir, Dfm, MutexGuard<'static, ()>) {
        let guard = ACTIVE_PROFILE_ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("DFM_PROFILE");
        }
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        (dir, ctx, guard)
    }

    #[test]
    fn create_profile_registers_config_entry_and_directory() {
        let (_dir, ctx, _guard) = ctx();

        let created = create_profile(&ctx, "work", Some("work laptop".to_string())).unwrap();
        assert_eq!(created.name, "work");
        assert_eq!(created.description, Some("work laptop".to_string()));
        assert!(created.directory.exists());
        assert_eq!(created.directory, ctx.profile_dir("work"));

        let config = ProfileConfig::load_or_default(&ctx);
        assert!(config.profile_exists("work"));
    }

    #[test]
    fn create_profile_backfills_empty_stored_version() {
        let (_dir, ctx, _guard) = ctx();

        let empty_version = ProfileConfig {
            version: String::new(),
            profiles: std::collections::HashMap::new(),
        };
        empty_version.save(&ctx.profiles_config_path()).unwrap();

        create_profile(&ctx, "work", None).unwrap();

        let config = ProfileConfig::load_or_default(&ctx);
        assert_eq!(config.version, "1.0.0");
    }

    #[test]
    fn create_profile_errors_when_directory_path_is_blocked_by_a_file() {
        let (_dir, ctx, _guard) = ctx();

        fs::create_dir_all(ctx.backup_dir().join("profiles")).unwrap();
        fs::write(ctx.profile_dir("work"), "not a directory").unwrap();

        let err = create_profile(&ctx, "work", None);

        assert!(err.is_err());
    }

    #[test]
    fn create_profile_rejects_duplicate_name() {
        let (_dir, ctx, _guard) = ctx();
        create_profile(&ctx, "work", None).unwrap();

        let err = create_profile(&ctx, "work", None).unwrap_err();
        assert!(matches!(err, Error::ProfileExists(name) if name == "work"));
    }

    #[test]
    fn create_profile_rejects_empty_name() {
        let (_dir, ctx, _guard) = ctx();
        let err = create_profile(&ctx, "", None).unwrap_err();
        assert!(matches!(err, Error::EmptyProfileName));
    }

    #[test]
    fn create_profile_rejects_invalid_characters() {
        let (_dir, ctx, _guard) = ctx();
        let err = create_profile(&ctx, "work profile!", None).unwrap_err();
        assert!(matches!(err, Error::InvalidProfileName(name) if name == "work profile!"));
    }

    #[test]
    fn create_profile_allows_hyphens_and_underscores() {
        let (_dir, ctx, _guard) = ctx();
        assert!(create_profile(&ctx, "work-laptop_2", None).is_ok());
    }

    #[test]
    fn delete_profile_removes_config_entry_but_keeps_directory() {
        let (_dir, ctx, _guard) = ctx();
        create_profile(&ctx, "work", None).unwrap();

        let deleted = delete_profile(&ctx, "work").unwrap();
        assert_eq!(deleted.name, "work");
        assert_eq!(deleted.retained_directory, Some(ctx.profile_dir("work")));
        assert!(ctx.profile_dir("work").exists());

        let config = ProfileConfig::load_or_default(&ctx);
        assert!(!config.profile_exists("work"));
    }

    #[test]
    fn delete_profile_missing_profile_errors() {
        let (_dir, ctx, _guard) = ctx();
        let err = delete_profile(&ctx, "missing").unwrap_err();
        assert!(matches!(err, Error::ProfileNotFound(name) if name == "missing"));
    }

    #[test]
    fn delete_profile_active_profile_errors() {
        let (_dir, ctx, _guard) = ctx();
        create_profile(&ctx, "work", None).unwrap();
        set_active_profile(&ctx, "work").unwrap();

        let err = delete_profile(&ctx, "work").unwrap_err();
        assert!(matches!(err, Error::DeleteActiveProfile(name) if name == "work"));
    }

    #[test]
    fn switch_profile_switches_to_existing_profile() {
        let (_dir, ctx, _guard) = ctx();
        create_profile(&ctx, "work", None).unwrap();

        let outcome = switch_profile(&ctx, "work").unwrap();
        assert!(matches!(outcome, SwitchProfileOutcome::Switched(name) if name == "work"));
        assert_eq!(get_active_profile_name(&ctx), Some("work".to_string()));
    }

    #[test]
    fn switch_profile_missing_profile_errors() {
        let (_dir, ctx, _guard) = ctx();
        let err = switch_profile(&ctx, "missing").unwrap_err();
        assert!(matches!(err, Error::ProfileNotFound(name) if name == "missing"));
    }

    #[test]
    fn switch_profile_already_active_returns_already_active() {
        let (_dir, ctx, _guard) = ctx();
        create_profile(&ctx, "work", None).unwrap();
        switch_profile(&ctx, "work").unwrap();

        let outcome = switch_profile(&ctx, "work").unwrap();
        assert!(matches!(outcome, SwitchProfileOutcome::AlreadyActive(name) if name == "work"));
    }

    #[test]
    fn switch_profile_common_name_clears_active_profile() {
        let (_dir, ctx, _guard) = ctx();
        create_profile(&ctx, "work", None).unwrap();
        switch_profile(&ctx, "work").unwrap();

        let outcome = switch_profile(&ctx, "common").unwrap();
        assert!(matches!(outcome, SwitchProfileOutcome::Cleared));
        assert_eq!(get_active_profile_name(&ctx), None);
    }

    #[test]
    fn switch_profile_none_name_clears_active_profile() {
        let (_dir, ctx, _guard) = ctx();
        create_profile(&ctx, "work", None).unwrap();
        switch_profile(&ctx, "work").unwrap();

        let outcome = switch_profile(&ctx, "none").unwrap();
        assert!(matches!(outcome, SwitchProfileOutcome::Cleared));
        assert_eq!(get_active_profile_name(&ctx), None);
    }
}
