use std::fs;
use std::path::PathBuf;

use super::config::ProfileConfig;
use crate::context::Dfm;
use crate::error::{Result, WrapErr};

/// A backup directory under `profiles_root_dir()` whose name has no matching
/// entry in `profiles.json` — left behind by a profile that was deleted (or
/// renamed) without also removing its backup directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanedProfile {
    pub name: String,
    pub directory: PathBuf,
}

/// A directory removed by [`prune_orphaned_profiles`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrunedProfile {
    pub name: String,
    pub directory: PathBuf,
}

/// Backup directories on disk with no matching profile, sorted by name.
/// Never touches disk beyond reading it.
pub fn find_orphaned_profiles(ctx: &Dfm) -> Result<Vec<OrphanedProfile>> {
    let profiles_root = ctx.profiles_root_dir();
    if !profiles_root.exists() {
        return Ok(Vec::new());
    }

    let config = ProfileConfig::load_or_default(ctx);
    let mut orphans = Vec::new();

    for entry in fs::read_dir(&profiles_root)
        .wrap_err_with(|| format!("Read profiles directory at {}", profiles_root.display()))?
    {
        let entry = entry
            .wrap_err_with(|| format!("Read entry in {}", profiles_root.display()))?;
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        if !config.profile_exists(&name) {
            orphans.push(OrphanedProfile {
                name,
                directory: entry.path(),
            });
        }
    }

    orphans.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(orphans)
}

/// Delete every orphaned profile directory found by
/// [`find_orphaned_profiles`], returning what was removed.
pub fn prune_orphaned_profiles(ctx: &Dfm) -> Result<Vec<PrunedProfile>> {
    let orphans = find_orphaned_profiles(ctx)?;
    let mut pruned = Vec::with_capacity(orphans.len());

    for orphan in orphans {
        fs::remove_dir_all(&orphan.directory).wrap_err_with(|| {
            format!("Remove orphaned profile directory {}", orphan.directory.display())
        })?;
        pruned.push(PrunedProfile {
            name: orphan.name,
            directory: orphan.directory,
        });
    }

    Ok(pruned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::create_profile;

    fn ctx() -> (tempfile::TempDir, Dfm) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        (dir, ctx)
    }

    #[test]
    fn find_orphaned_profiles_returns_empty_when_root_missing() {
        let (_dir, ctx) = ctx();
        assert_eq!(find_orphaned_profiles(&ctx).unwrap(), vec![]);
    }

    #[test]
    fn find_orphaned_profiles_ignores_known_profiles() {
        let (_dir, ctx) = ctx();
        create_profile(&ctx, "work", None).unwrap();
        assert_eq!(find_orphaned_profiles(&ctx).unwrap(), vec![]);
    }

    #[test]
    fn find_orphaned_profiles_finds_directories_with_no_matching_profile() {
        let (_dir, ctx) = ctx();
        create_profile(&ctx, "work", None).unwrap();
        fs::create_dir_all(ctx.profile_dir("stale")).unwrap();

        let orphans = find_orphaned_profiles(&ctx).unwrap();
        assert_eq!(
            orphans,
            vec![OrphanedProfile {
                name: "stale".to_string(),
                directory: ctx.profile_dir("stale"),
            }]
        );
    }

    #[test]
    fn find_orphaned_profiles_ignores_stray_files() {
        let (_dir, ctx) = ctx();
        fs::create_dir_all(ctx.profiles_root_dir()).unwrap();
        fs::write(ctx.profiles_root_dir().join("not-a-dir.txt"), "x").unwrap();

        assert_eq!(find_orphaned_profiles(&ctx).unwrap(), vec![]);
    }

    #[test]
    fn find_orphaned_profiles_sorted_by_name() {
        let (_dir, ctx) = ctx();
        fs::create_dir_all(ctx.profile_dir("zeta")).unwrap();
        fs::create_dir_all(ctx.profile_dir("alpha")).unwrap();

        let names: Vec<_> = find_orphaned_profiles(&ctx)
            .unwrap()
            .into_iter()
            .map(|o| o.name)
            .collect();
        assert_eq!(names, vec!["alpha", "zeta"]);
    }

    #[test]
    fn prune_orphaned_profiles_deletes_directories_and_reports_them() {
        let (_dir, ctx) = ctx();
        create_profile(&ctx, "work", None).unwrap();
        fs::create_dir_all(ctx.profile_dir("stale")).unwrap();

        let pruned = prune_orphaned_profiles(&ctx).unwrap();

        assert_eq!(
            pruned,
            vec![PrunedProfile {
                name: "stale".to_string(),
                directory: ctx.profile_dir("stale"),
            }]
        );
        assert!(!ctx.profile_dir("stale").exists());
        assert!(ctx.profile_dir("work").exists());
    }

    #[test]
    fn prune_orphaned_profiles_removes_nested_encrypted_contents() {
        let (_dir, ctx) = ctx();
        fs::create_dir_all(ctx.encrypted_profile_dir("stale")).unwrap();
        fs::write(
            ctx.encrypted_profile_dir("stale").join("secret.age"),
            "x",
        )
        .unwrap();

        prune_orphaned_profiles(&ctx).unwrap();

        assert!(!ctx.profile_dir("stale").exists());
    }

    #[test]
    fn prune_orphaned_profiles_is_noop_when_none_found() {
        let (_dir, ctx) = ctx();
        create_profile(&ctx, "work", None).unwrap();
        assert_eq!(prune_orphaned_profiles(&ctx).unwrap(), vec![]);
    }
}
