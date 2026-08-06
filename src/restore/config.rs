use std::fs;
use std::path::Path;

use crate::utils::fs::sync_directory_contents;

/// Restore one backup file or directory to its original location. Returns a
/// human-readable reason on failure.
pub(super) fn restore_config(
    backup_path: &Path,
    original_path: &Path,
) -> std::result::Result<(), String> {
    if backup_path.is_dir() {
        return restore_directory(backup_path, original_path);
    }

    let contents = fs::read(backup_path).map_err(|e| {
        format!(
            "failed to read backup file {}: {}",
            backup_path.display(),
            e
        )
    })?;

    if let Some(parent) = original_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directory {}: {}", parent.display(), e))?;
    }

    fs::write(original_path, contents)
        .map_err(|e| format!("failed to write {}: {}", original_path.display(), e))
}

/// Mirror `backup_path`'s contents into `original_path`.
fn restore_directory(backup_path: &Path, original_path: &Path) -> std::result::Result<(), String> {
    fs::create_dir_all(original_path).map_err(|e| {
        format!(
            "failed to create original directory {}: {}",
            original_path.display(),
            e
        )
    })?;

    sync_directory_contents(backup_path, original_path).map_err(|e| {
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

        let original_path = dir.path().join("nested/deep/original.txt");

        restore_config(&backup_path, &original_path).unwrap();

        assert_eq!(fs::read(&original_path).unwrap(), b"restored content");
    }

    #[test]
    fn restores_a_directory_mirroring_nested_contents() {
        let dir = tempfile::tempdir().unwrap();
        let backup_path = dir.path().join("backup_dir");
        fs::create_dir_all(backup_path.join("sub")).unwrap();
        fs::write(backup_path.join("top.txt"), b"top").unwrap();
        fs::write(backup_path.join("sub/inner.txt"), b"inner").unwrap();

        let original_path = dir.path().join("original_dir");

        restore_config(&backup_path, &original_path).unwrap();

        assert_eq!(fs::read(original_path.join("top.txt")).unwrap(), b"top");
        assert_eq!(
            fs::read(original_path.join("sub/inner.txt")).unwrap(),
            b"inner"
        );
    }

    #[test]
    fn restoring_a_directory_prunes_stale_original_entries() {
        let dir = tempfile::tempdir().unwrap();
        let backup_path = dir.path().join("backup_dir");
        fs::create_dir_all(&backup_path).unwrap();
        fs::write(backup_path.join("keep.txt"), b"keep").unwrap();

        let original_path = dir.path().join("original_dir");
        fs::create_dir_all(&original_path).unwrap();
        fs::write(original_path.join("stale.txt"), b"stale").unwrap();

        restore_config(&backup_path, &original_path).unwrap();

        assert!(original_path.join("keep.txt").exists());
        assert!(!original_path.join("stale.txt").exists());
    }

    #[test]
    fn missing_backup_file_returns_readable_error() {
        let dir = tempfile::tempdir().unwrap();
        let backup_path = dir.path().join("missing.txt");
        let original_path = dir.path().join("original.txt");

        let err = restore_config(&backup_path, &original_path).unwrap_err();
        assert!(err.contains("failed to read backup file"));
    }
}
