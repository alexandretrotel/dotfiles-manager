use std::path::{Component, Path, PathBuf};

use super::ActiveProfile;
use crate::context::{Dfm, ENCRYPTED_BUNDLE_FILE};

/// Which backup layer a source file was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLayer {
    Common,
    Profile,
}

impl std::fmt::Display for SourceLayer {
    /// Prints as `"common"` or `"profile"`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceLayer::Common => write!(f, "common"),
            SourceLayer::Profile => write!(f, "profile"),
        }
    }
}

/// A backup file found on disk, together with which layer it came from.
#[derive(Debug, Clone)]
pub struct ResolvedSource {
    pub path: PathBuf,
    pub layer: SourceLayer,
}

impl ActiveProfile {
    /// First existing backup of `source_path`, profile layer first.
    pub fn resolve_source(&self, ctx: &Dfm, source_path: &str) -> Option<ResolvedSource> {
        Self::first_existing(self.get_candidate_sources(ctx, source_path))
    }

    /// Where `source_path` could live, profile layer first (whether or not
    /// each candidate actually exists on disk).
    pub fn get_candidate_sources(
        &self,
        ctx: &Dfm,
        source_path: &str,
    ) -> Vec<(PathBuf, SourceLayer)> {
        self.candidate_paths(ctx, source_path, Dfm::profile_dir, Dfm::common_dir)
    }

    /// Every existing backup of `source_path`, across all layers.
    pub fn get_all_resolved_sources(&self, ctx: &Dfm, source_path: &str) -> Vec<ResolvedSource> {
        Self::all_existing(self.get_candidate_sources(ctx, source_path))
    }

    /// First existing encrypted bundle, profile layer first.
    pub fn resolve_encrypted_bundle(&self, ctx: &Dfm) -> Option<ResolvedSource> {
        self.resolve_encrypted_source(ctx, ENCRYPTED_BUNDLE_FILE)
    }

    /// First existing encrypted backup of `source_path`, profile layer first.
    pub fn resolve_encrypted_source(&self, ctx: &Dfm, source_path: &str) -> Option<ResolvedSource> {
        Self::first_existing(self.get_candidate_encrypted_sources(ctx, source_path))
    }

    /// Where the encrypted backup of `source_path` could live, profile layer
    /// first (whether or not each candidate actually exists on disk).
    pub fn get_candidate_encrypted_sources(
        &self,
        ctx: &Dfm,
        source_path: &str,
    ) -> Vec<(PathBuf, SourceLayer)> {
        self.candidate_paths(
            ctx,
            source_path,
            Dfm::encrypted_profile_dir,
            Dfm::encrypted_common_dir,
        )
    }

    /// Profile layer (if any) then common layer, for whichever pair of
    /// directory getters the caller passes (plain or encrypted).
    fn candidate_paths(
        &self,
        ctx: &Dfm,
        source_path: &str,
        profile_dir: impl Fn(&Dfm, &str) -> PathBuf,
        common_dir: impl Fn(&Dfm) -> PathBuf,
    ) -> Vec<(PathBuf, SourceLayer)> {
        if !is_valid_source_path(source_path) {
            return Vec::new();
        }

        let mut candidates = Vec::new();

        if let Some(profile_name) = &self.name {
            candidates.push((
                profile_dir(ctx, profile_name).join(source_path),
                SourceLayer::Profile,
            ));
        }

        candidates.push((common_dir(ctx).join(source_path), SourceLayer::Common));
        candidates
    }

    /// First candidate that exists on disk.
    fn first_existing(candidates: Vec<(PathBuf, SourceLayer)>) -> Option<ResolvedSource> {
        candidates
            .into_iter()
            .find(|(path, _)| path.exists())
            .map(|(path, layer)| ResolvedSource { path, layer })
    }

    /// All candidates that exist on disk.
    fn all_existing(candidates: Vec<(PathBuf, SourceLayer)>) -> Vec<ResolvedSource> {
        candidates
            .into_iter()
            .filter(|(path, _)| path.exists())
            .map(|(path, layer)| ResolvedSource { path, layer })
            .collect()
    }
}

/// Rejects empty paths, absolute paths, and paths with `..` components —
/// `source_path` is later joined onto a backup directory, so it must not be
/// able to escape it.
fn is_valid_source_path(source_path: &str) -> bool {
    if source_path.is_empty() {
        return false;
    }

    let path = Path::new(source_path);
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
    fn is_valid_source_path_rejects_empty() {
        assert!(!is_valid_source_path(""));
    }

    #[test]
    fn is_valid_source_path_rejects_absolute() {
        assert!(!is_valid_source_path("/etc/passwd"));
    }

    #[test]
    fn is_valid_source_path_rejects_parent_dir_component() {
        assert!(!is_valid_source_path("../secrets"));
        assert!(!is_valid_source_path("foo/../../secrets"));
    }

    #[test]
    fn is_valid_source_path_accepts_relative_path() {
        assert!(is_valid_source_path(".config/nvim/init.lua"));
    }

    #[test]
    fn source_layer_display() {
        assert_eq!(SourceLayer::Common.to_string(), "common");
        assert_eq!(SourceLayer::Profile.to_string(), "profile");
    }

    #[test]
    fn resolve_source_prefers_profile_layer_over_common() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");

        write_file(ctx.common_dir().join(".zshrc"), "common");
        write_file(ctx.profile_dir("work").join(".zshrc"), "profile");

        let resolved = profile.resolve_source(&ctx, ".zshrc").unwrap();
        assert_eq!(resolved.layer, SourceLayer::Profile);
        assert_eq!(resolved.path, ctx.profile_dir("work").join(".zshrc"));
    }

    #[test]
    fn resolve_source_falls_back_to_common_when_profile_missing() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");

        write_file(ctx.common_dir().join(".zshrc"), "common");

        let resolved = profile.resolve_source(&ctx, ".zshrc").unwrap();
        assert_eq!(resolved.layer, SourceLayer::Common);
        assert_eq!(resolved.path, ctx.common_dir().join(".zshrc"));
    }

    #[test]
    fn resolve_source_returns_none_when_missing_everywhere() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");
        assert!(profile.resolve_source(&ctx, ".zshrc").is_none());
    }

    #[test]
    fn resolve_source_rejects_invalid_source_path() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::common_only();
        write_file(ctx.common_dir().join("secrets"), "hi");
        assert!(profile.resolve_source(&ctx, "../secrets").is_none());
    }

    #[test]
    fn get_candidate_sources_includes_profile_then_common_when_named() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");

        let candidates = profile.get_candidate_sources(&ctx, ".zshrc");
        assert_eq!(
            candidates,
            vec![
                (ctx.profile_dir("work").join(".zshrc"), SourceLayer::Profile),
                (ctx.common_dir().join(".zshrc"), SourceLayer::Common),
            ]
        );
    }

    #[test]
    fn get_candidate_sources_common_only_when_unnamed() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::common_only();

        let candidates = profile.get_candidate_sources(&ctx, ".zshrc");
        assert_eq!(
            candidates,
            vec![(ctx.common_dir().join(".zshrc"), SourceLayer::Common)]
        );
    }

    #[test]
    fn get_all_resolved_sources_returns_every_existing_layer() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");

        write_file(ctx.common_dir().join(".zshrc"), "common");
        write_file(ctx.profile_dir("work").join(".zshrc"), "profile");

        let resolved = profile.get_all_resolved_sources(&ctx, ".zshrc");
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].layer, SourceLayer::Profile);
        assert_eq!(resolved[1].layer, SourceLayer::Common);
    }

    #[test]
    fn get_all_resolved_sources_skips_missing_layers() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");

        write_file(ctx.common_dir().join(".zshrc"), "common");

        let resolved = profile.get_all_resolved_sources(&ctx, ".zshrc");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].layer, SourceLayer::Common);
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
        assert_eq!(resolved.layer, SourceLayer::Profile);
        assert_eq!(
            resolved.path,
            ctx.encrypted_profile_dir("work")
                .join(ENCRYPTED_BUNDLE_FILE)
        );
    }

    #[test]
    fn resolve_encrypted_source_prefers_profile_layer_over_common() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::with_profile("work");

        write_file(ctx.encrypted_common_dir().join("secret.age"), "common");
        write_file(
            ctx.encrypted_profile_dir("work").join("secret.age"),
            "profile",
        );

        let resolved = profile
            .resolve_encrypted_source(&ctx, "secret.age")
            .unwrap();
        assert_eq!(resolved.layer, SourceLayer::Profile);
    }

    #[test]
    fn resolve_encrypted_source_returns_none_when_missing() {
        let (_dir, ctx) = dfm();
        let profile = ActiveProfile::common_only();
        assert!(
            profile
                .resolve_encrypted_source(&ctx, "secret.age")
                .is_none()
        );
    }
}
