use std::fs;
use std::path::Path;

use crate::utils::fs::sync_directory_contents;

/// Restore one backup file or directory to its target. Returns a
/// human-readable reason on failure.
pub(super) fn restore_config(
    backup_path: &Path,
    target_path: &Path,
) -> std::result::Result<(), String> {
    if backup_path.is_dir() {
        return restore_directory(backup_path, target_path);
    }

    let contents = fs::read(backup_path).map_err(|e| {
        format!(
            "failed to read backup file {}: {}",
            backup_path.display(),
            e
        )
    })?;

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directory {}: {}", parent.display(), e))?;
    }

    fs::write(target_path, contents)
        .map_err(|e| format!("failed to write {}: {}", target_path.display(), e))
}

/// Mirror `backup_path`'s contents into `target_path`.
fn restore_directory(backup_path: &Path, target_path: &Path) -> std::result::Result<(), String> {
    fs::create_dir_all(target_path).map_err(|e| {
        format!(
            "failed to create target directory {}: {}",
            target_path.display(),
            e
        )
    })?;

    sync_directory_contents(backup_path, target_path).map_err(|e| {
        format!(
            "failed to restore directory {}: {}",
            backup_path.display(),
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_a_single_file_creating_missing_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let backup_path = dir.path().join("backup.txt");
        fs::write(&backup_path, b"restored content").unwrap();

        let target_path = dir.path().join("nested/deep/target.txt");

        restore_config(&backup_path, &target_path).unwrap();

        assert_eq!(fs::read(&target_path).unwrap(), b"restored content");
    }

    #[test]
    fn restores_a_directory_mirroring_nested_contents() {
        let dir = tempfile::tempdir().unwrap();
        let backup_path = dir.path().join("backup_dir");
        fs::create_dir_all(backup_path.join("sub")).unwrap();
        fs::write(backup_path.join("top.txt"), b"top").unwrap();
        fs::write(backup_path.join("sub/inner.txt"), b"inner").unwrap();

        let target_path = dir.path().join("target_dir");

        restore_config(&backup_path, &target_path).unwrap();

        assert_eq!(fs::read(target_path.join("top.txt")).unwrap(), b"top");
        assert_eq!(
            fs::read(target_path.join("sub/inner.txt")).unwrap(),
            b"inner"
        );
    }

    #[test]
    fn restoring_a_directory_prunes_stale_target_entries() {
        let dir = tempfile::tempdir().unwrap();
        let backup_path = dir.path().join("backup_dir");
        fs::create_dir_all(&backup_path).unwrap();
        fs::write(backup_path.join("keep.txt"), b"keep").unwrap();

        let target_path = dir.path().join("target_dir");
        fs::create_dir_all(&target_path).unwrap();
        fs::write(target_path.join("stale.txt"), b"stale").unwrap();

        restore_config(&backup_path, &target_path).unwrap();

        assert!(target_path.join("keep.txt").exists());
        assert!(!target_path.join("stale.txt").exists());
    }

    #[test]
    fn missing_backup_file_returns_readable_error() {
        let dir = tempfile::tempdir().unwrap();
        let backup_path = dir.path().join("missing.txt");
        let target_path = dir.path().join("target.txt");

        let err = restore_config(&backup_path, &target_path).unwrap_err();
        assert!(err.contains("failed to read backup file"));
    }
}
