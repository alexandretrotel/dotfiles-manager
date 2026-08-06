use std::path::{Component, Path, PathBuf};

use super::ActiveProfile;
use crate::context::{Dfm, ENCRYPTED_BUNDLE_FILE};

/// Which backup layer a backup file was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupLayer {
    Common,
    Profile,
}

impl std::fmt::Display for BackupLayer {
    /// Prints as `"common"` or `"profile"`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupLayer::Common => write!(f, "common"),
            BackupLayer::Profile => write!(f, "profile"),
        }
    }
}

/// A backup file found on disk, together with which layer it came from.
#[derive(Debug, Clone)]
pub struct ResolvedBackup {
    pub path: PathBuf,
    pub layer: BackupLayer,
}

impl ActiveProfile {
    /// First existing backup of `backup_path`, profile layer first.
    pub fn resolve_backup_path(&self, ctx: &Dfm, backup_path: &str) -> Option<ResolvedBackup> {
        Self::first_existing(self.get_candidate_backup_paths(ctx, backup_path))
    }

    /// Where `backup_path` could live, profile layer first (whether or not
    /// each candidate actually exists on disk).
    fn get_candidate_backup_paths(
        &self,
        ctx: &Dfm,
        backup_path: &str,
    ) -> Vec<(PathBuf, BackupLayer)> {
        self.candidate_paths(ctx, backup_path, Dfm::profile_dir, Dfm::common_dir)
    }

    /// First existing encrypted bundle, profile layer first.
    pub fn resolve_encrypted_bundle(&self, ctx: &Dfm) -> Option<ResolvedBackup> {
        Self::first_existing(self.get_candidate_encrypted_backup_paths(ctx, ENCRYPTED_BUNDLE_FILE))
    }

    /// Where the encrypted backup of `backup_path` could live, profile layer
    /// first (whether or not each candidate actually exists on disk).
    fn get_candidate_encrypted_backup_paths(
        &self,
        ctx: &Dfm,
        backup_path: &str,
    ) -> Vec<(PathBuf, BackupLayer)> {
        self.candidate_paths(
            ctx,
            backup_path,
            Dfm::encrypted_profile_dir,
            Dfm::encrypted_common_dir,
        )
    }

    /// Profile layer (if any) then common layer, for whichever pair of
    /// directory getters the caller passes (plain or encrypted).
    fn candidate_paths(
        &self,
        ctx: &Dfm,
        backup_path: &str,
        profile_dir: impl Fn(&Dfm, &str) -> PathBuf,
        common_dir: impl Fn(&Dfm) -> PathBuf,
    ) -> Vec<(PathBuf, BackupLayer)> {
        if !is_valid_backup_path(backup_path) {
            return Vec::new();
        }

        let mut candidates = Vec::new();

        if let Some(profile_name) = &self.name {
            candidates.push((
                profile_dir(ctx, profile_name).join(backup_path),
                BackupLayer::Profile,
            ));
        }

        candidates.push((common_dir(ctx).join(backup_path), BackupLayer::Common));
        candidates
    }

    /// First candidate that exists on disk.
    fn first_existing(candidates: Vec<(PathBuf, BackupLayer)>) -> Option<ResolvedBackup> {
        candidates
            .into_iter()
            .find(|(path, _)| path.exists())
            .map(|(path, layer)| ResolvedBackup { path, layer })
    }
}

/// Rejects empty paths, absolute paths, and paths with `..` components —
/// `backup_path` is later joined onto a backup directory, so it must not be
/// able to escape it.
fn is_valid_backup_path(backup_path: &str) -> bool {
    if backup_path.is_empty() {
        return false;
    }

    let path = Path::new(backup_path);
    !path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn dfm() -> (tempfile::TempDir, Dfm) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        (dir, ctx)
    }

    fn write_file(path: PathBuf, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn is_valid_backup_path_rejects_empty() {
        assert!(!is_valid_backup_path(""));
    }

    #[test]
    fn is_valid_backup_path_rejects_absolute() {
        assert!(!is_valid_backup_path("/etc/passwd"));
    }

    #[test]
    fn is_valid_backup_path_rejects_parent_dir_component() {
        assert!(!is_valid_backup_path("../secrets"));
        assert!(!is_valid_backup_path("foo/../../secrets"));
    }

    #[test]
    fn is_valid_backup_path_accepts_relative_path() {
        assert!(is_valid_backup_path(".config/nvim/init.lua"));
    }

    #[test]
    fn backup_layer_display() {
        assert_eq!(BackupLayer::Common.to_string(), "common");
        assert_eq!(BackupLayer::Profile.to_string(), "profile");
    }

    #[test]
    fn resolve_backup_path_prefers_profile_layer_over_common() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");

        write_file(ctx.common_dir().join(".zshrc"), "common");
        write_file(ctx.profile_dir("work").join(".zshrc"), "profile");

        let resolved = profile.resolve_backup_path(&ctx, ".zshrc").unwrap();
        assert_eq!(resolved.layer, BackupLayer::Profile);
        assert_eq!(resolved.path, ctx.profile_dir("work").join(".zshrc"));
    }

    #[test]
    fn resolve_backup_path_falls_back_to_common_when_profile_missing() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");

        write_file(ctx.common_dir().join(".zshrc"), "common");

        let resolved = profile.resolve_backup_path(&ctx, ".zshrc").unwrap();
        assert_eq!(resolved.layer, BackupLayer::Common);
        assert_eq!(resolved.path, ctx.common_dir().join(".zshrc"));
    }

    #[test]
    fn resolve_backup_path_returns_none_when_missing_everywhere() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");
        assert!(profile.resolve_backup_path(&ctx, ".zshrc").is_none());
    }

    #[test]
    fn resolve_backup_path_rejects_invalid_backup_path() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::common_only();
        write_file(ctx.common_dir().join("secrets"), "hi");
        assert!(profile.resolve_backup_path(&ctx, "../secrets").is_none());
    }

    #[test]
    fn get_candidate_backup_paths_includes_profile_then_common_when_named() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");

        let candidates = profile.get_candidate_backup_paths(&ctx, ".zshrc");
        assert_eq!(
            candidates,
            vec![
                (ctx.profile_dir("work").join(".zshrc"), BackupLayer::Profile),
                (ctx.common_dir().join(".zshrc"), BackupLayer::Common),
            ]
        );
    }

    #[test]
    fn get_candidate_backup_paths_common_only_when_unnamed() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::common_only();

        let candidates = profile.get_candidate_backup_paths(&ctx, ".zshrc");
        assert_eq!(
            candidates,
            vec![(ctx.common_dir().join(".zshrc"), BackupLayer::Common)]
        );
    }

    #[test]
    fn resolve_encrypted_bundle_finds_bundle_in_profile_layer() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");

        write_file(
            ctx.encrypted_profile_dir("work")
                .join(ENCRYPTED_BUNDLE_FILE),
            "bundle",
        );

        let resolved = profile.resolve_encrypted_bundle(&ctx).unwrap();
        assert_eq!(resolved.layer, BackupLayer::Profile);
        assert_eq!(
            resolved.path,
            ctx.encrypted_profile_dir("work")
                .join(ENCRYPTED_BUNDLE_FILE)
        );
    }
}
