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
