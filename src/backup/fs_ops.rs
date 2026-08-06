use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::fs::{copy_dir_recursive, sync_directory_contents};

/// Fail with a `NotFound` error unless `source` exists.
fn require_exists(source: &Path, kind: &str) -> std::io::Result<()> {
    if source.exists() {
        return Ok(());
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Source {kind} {} not found", source.display()),
    ))
}

/// If `source` is a symlink whose target canonicalizes to `destination`,
/// return that canonical target: the backup was already copied there on a
/// previous run, so `source` needs converting back into a real file/dir.
fn symlink_self_reference(source: &Path, destination: &Path) -> Option<PathBuf> {
    if !source.is_symlink() {
        return None;
    }

    let link_target = fs::read_link(source).ok()?;
    let canonical_target = link_target
        .canonicalize()
        .unwrap_or_else(|_| link_target.clone());
    let canonical_dest = destination
        .canonicalize()
        .unwrap_or_else(|_| destination.to_path_buf());

    (canonical_target == canonical_dest).then_some(canonical_target)
}

/// Copy a file into the backup. When the source is a symlink pointing at its
/// own backup destination, it is converted back into a real file first; the
/// returned note describes that conversion.
pub(super) fn backup_file(source: &Path, destination: &Path) -> std::io::Result<Option<String>> {
    require_exists(source, "file")?;

    if let Some(canonical_target) = symlink_self_reference(source, destination) {
        let content = fs::read(&canonical_target)?;
        fs::remove_file(source)?;
        fs::write(source, &content)?;
        return Ok(Some(format!(
            "Converted symlink to real file: {}",
            source.display()
        )));
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(source, destination)?;
    Ok(None)
}

/// Copy a directory into the backup; same symlink conversion behavior as
/// [`backup_file`].
pub(super) fn backup_directory(
    source: &Path,
    destination: &Path,
) -> std::io::Result<Option<String>> {
    require_exists(source, "directory")?;

    if let Some(canonical_target) = symlink_self_reference(source, destination) {
        let file_type = fs::symlink_metadata(source)?.file_type();
        if file_type.is_dir() || (file_type.is_symlink() && fs::metadata(source)?.is_dir()) {
            fs::remove_dir(source)?;
        } else {
            fs::remove_file(source)?;
        }
        fs::create_dir_all(source)?;
        copy_dir_recursive(&canonical_target, source)?;
        return Ok(Some(format!(
            "Converted symlink to real directory: {}",
            source.display()
        )));
    }

    fs::create_dir_all(destination)?;
    sync_directory_contents(source, destination)?;
    Ok(None)
}
