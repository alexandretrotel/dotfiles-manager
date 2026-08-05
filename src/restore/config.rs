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
