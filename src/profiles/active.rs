use std::fs;
use std::io;
use std::path::PathBuf;

use crate::context::Dfm;

/// The profile a backup or restore run targets: either a named profile
/// layered on top of `common`, or `common` alone.
#[derive(Debug, Clone)]
pub struct ActiveProfile {
    pub name: Option<String>,
}

impl std::fmt::Display for ActiveProfile {
    /// Prints as `profile=<name>`, or `common (no active profile)`.
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
    /// `DFM_PROFILE` environment variable, then the persisted active
    /// profile, then `common`.
    pub fn resolve(ctx: &Dfm, override_profile: Option<&str>) -> Self {
        if let Some(profile) = override_profile {
            return Self::with_profile(profile);
        }

        if let Some(profile) = get_active_profile_name(ctx) {
            return Self::with_profile(&profile);
        }

        Self::common_only()
    }

    /// This profile's backup directory (profile layer or `common`).
    pub fn backup_path(&self, ctx: &Dfm) -> PathBuf {
        match &self.name {
            Some(name) => ctx.profile_dir(name),
            None => ctx.common_dir(),
        }
    }

    /// This profile's encrypted-files directory (profile layer or `common`).
    pub fn encrypted_backup_path(&self, ctx: &Dfm) -> PathBuf {
        match &self.name {
            Some(name) => ctx.encrypted_profile_dir(name),
            None => ctx.encrypted_common_dir(),
        }
    }
}

/// Name of the active profile, from `DFM_PROFILE` or the persisted
/// `.active-profile` file.
pub fn get_active_profile_name(ctx: &Dfm) -> Option<String> {
    if let Ok(profile) = std::env::var("DFM_PROFILE") {
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
pub fn set_active_profile(ctx: &Dfm, profile_name: &str) -> io::Result<()> {
    let active_profile_path = ctx.active_profile_path();
    if let Some(parent) = active_profile_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(active_profile_path, profile_name)
}

/// Remove the persisted active profile, if any.
pub fn clear_active_profile(ctx: &Dfm) -> io::Result<()> {
    let active_profile_path = ctx.active_profile_path();
    if active_profile_path.exists() {
        fs::remove_file(active_profile_path)?;
    }
    Ok(())
}

// `DFM_PROFILE` is process-global state; every test in this crate that reads
// or writes it (directly, or transitively via `resolve`/
// `get_active_profile_name`) locks this first so they don't race each other.
#[cfg(test)]
pub(crate) static ACTIVE_PROFILE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_env() {
        unsafe {
            std::env::remove_var("DFM_PROFILE");
        }
    }

    #[test]
    fn with_profile_sets_name() {
        assert_eq!(
            ActiveProfile::with_profile("work").name,
            Some("work".to_string())
        );
    }

    #[test]
    fn common_only_has_no_name() {
        assert_eq!(ActiveProfile::common_only().name, None);
    }

    #[test]
    fn display_shows_profile_name() {
        assert_eq!(
            ActiveProfile::with_profile("work").to_string(),
            "profile=work"
        );
    }

    #[test]
    fn display_shows_common_when_no_profile() {
        assert_eq!(
            ActiveProfile::common_only().to_string(),
            "common (no active profile)"
        );
    }

    #[test]
    fn backup_path_uses_profile_dir_when_named() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        assert_eq!(
            ActiveProfile::with_profile("work").backup_path(&ctx),
            ctx.profile_dir("work")
        );
    }

    #[test]
    fn backup_path_uses_common_dir_when_unnamed() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        assert_eq!(
            ActiveProfile::common_only().backup_path(&ctx),
            ctx.common_dir()
        );
    }

    #[test]
    fn encrypted_backup_path_uses_encrypted_profile_dir_when_named() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        assert_eq!(
            ActiveProfile::with_profile("work").encrypted_backup_path(&ctx),
            ctx.encrypted_profile_dir("work")
        );
    }

    #[test]
    fn encrypted_backup_path_uses_encrypted_common_dir_when_unnamed() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        assert_eq!(
            ActiveProfile::common_only().encrypted_backup_path(&ctx),
            ctx.encrypted_common_dir()
        );
    }

    #[test]
    fn set_get_clear_active_profile_round_trip() {
        let _guard = ACTIVE_PROFILE_ENV_LOCK.lock().unwrap();
        clear_env();
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        assert_eq!(get_active_profile_name(&ctx), None);

        set_active_profile(&ctx, "work").unwrap();
        assert_eq!(get_active_profile_name(&ctx), Some("work".to_string()));

        clear_active_profile(&ctx).unwrap();
        assert_eq!(get_active_profile_name(&ctx), None);
    }

    #[test]
    fn clear_active_profile_is_noop_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        assert!(clear_active_profile(&ctx).is_ok());
    }

    #[test]
    fn get_active_profile_name_ignores_blank_file_content() {
        let _guard = ACTIVE_PROFILE_ENV_LOCK.lock().unwrap();
        clear_env();
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        fs::create_dir_all(ctx.root()).unwrap();
        fs::write(ctx.active_profile_path(), "   \n").unwrap();
        assert_eq!(get_active_profile_name(&ctx), None);
    }

    #[test]
    fn resolve_prefers_override_over_active_file() {
        let _guard = ACTIVE_PROFILE_ENV_LOCK.lock().unwrap();
        clear_env();
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        set_active_profile(&ctx, "file-profile").unwrap();

        let resolved = ActiveProfile::resolve(&ctx, Some("override-profile"));
        assert_eq!(resolved.name, Some("override-profile".to_string()));
    }

    #[test]
    fn resolve_falls_back_to_active_profile_file() {
        let _guard = ACTIVE_PROFILE_ENV_LOCK.lock().unwrap();
        clear_env();
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        set_active_profile(&ctx, "work").unwrap();

        let resolved = ActiveProfile::resolve(&ctx, None);
        assert_eq!(resolved.name, Some("work".to_string()));
    }

    #[test]
    fn resolve_falls_back_to_common_when_nothing_set() {
        let _guard = ACTIVE_PROFILE_ENV_LOCK.lock().unwrap();
        clear_env();
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        let resolved = ActiveProfile::resolve(&ctx, None);
        assert_eq!(resolved.name, None);
    }

    #[test]
    fn resolve_prefers_env_var_over_active_profile_file() {
        let _guard = ACTIVE_PROFILE_ENV_LOCK.lock().unwrap();
        clear_env();

        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        set_active_profile(&ctx, "file-profile").unwrap();
        unsafe {
            std::env::set_var("DFM_PROFILE", "env-profile");
        }

        let resolved = ActiveProfile::resolve(&ctx, None);
        clear_env();

        assert_eq!(resolved.name, Some("env-profile".to_string()));
    }

    #[test]
    fn get_active_profile_name_ignores_blank_env_var() {
        let _guard = ACTIVE_PROFILE_ENV_LOCK.lock().unwrap();
        clear_env();

        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        set_active_profile(&ctx, "work").unwrap();
        unsafe {
            std::env::set_var("DFM_PROFILE", "   ");
        }

        let name = get_active_profile_name(&ctx);
        clear_env();

        assert_eq!(name, Some("work".to_string()));
    }
}
