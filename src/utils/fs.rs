use std::{collections::HashSet, ffi::OsString, fs, io, path::Path, path::PathBuf};

/// Recursively copies `source` dir into `destination`, skipping symlinks.
pub(crate) fn copy_dir_recursive(source: &Path, destination: &Path) -> io::Result<()> {
    copy_dir(source, destination, None)
}

/// Fail with a `NotFound` error unless `source` exists.
fn require_exists(source: &Path, kind: &str) -> io::Result<()> {
    if source.exists() {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Source {kind} {} not found", source.display()),
    ))
}

/// If `source` is a symlink whose target canonicalizes to `destination`,
/// return that canonical target: a previous sync already placed the real
/// content at `destination` and left `source` pointing at it, so `source`
/// needs converting back into a real file/dir.
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

/// Copy a file from `source` to `destination`. When `source` is a symlink
/// pointing at its own `destination`, it is converted back into a real file
/// first; the returned note describes that conversion.
pub(crate) fn sync_file(source: &Path, destination: &Path) -> io::Result<Option<String>> {
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

/// Mirror `source` directory into `destination`; same symlink conversion
/// behavior as [`sync_file`].
pub(crate) fn sync_directory(source: &Path, destination: &Path) -> io::Result<Option<String>> {
    require_exists(source, "directory")?;

    if let Some(canonical_target) = symlink_self_reference(source, destination) {
        fs::remove_file(source)?;
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

/// Mirrors `source` contents into `destination`, deleting destination entries absent from source.
///
/// Recurses into shared subdirectories so stale files nested arbitrarily deep are
/// also removed.
pub(crate) fn sync_directory_contents(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    let mut seen = HashSet::new();
    copy_dir(source, destination, Some(&mut seen))?;
    prune_extraneous(destination, &seen)
}

/// Copies `source` into `destination`, skipping symlinks. When `seen` is given, every
/// copied entry's name is recorded into it and subdirectories are mirrored
/// (stale entries pruned) rather than plain-copied.
fn copy_dir(
    source: &Path,
    destination: &Path,
    mut seen: Option<&mut HashSet<OsString>>,
) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let source_path = entry.path();
        let destination_path = destination.join(&name);

        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            continue;
        }

        if let Some(seen) = seen.as_deref_mut() {
            seen.insert(name);
        }

        if metadata.is_dir() {
            fs::create_dir_all(&destination_path)?;
            match seen.as_deref_mut() {
                Some(_) => sync_directory_contents(&source_path, &destination_path)?,
                None => copy_dir(&source_path, &destination_path, None)?,
            }
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

/// Removes any entry under `dir` whose name isn't in `keep`.
fn prune_extraneous(dir: &Path, keep: &HashSet<OsString>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if keep.contains(&entry.file_name()) {
            continue;
        }

        let path = entry.path();
        if fs::symlink_metadata(&path)?.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_dir_recursive_copies_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let destination = dir.path().join("destination");

        fs::create_dir_all(source.join("sub")).unwrap();
        fs::write(source.join("top.txt"), "top").unwrap();
        fs::write(source.join("sub/nested.txt"), "nested").unwrap();
        fs::create_dir_all(&destination).unwrap();

        copy_dir_recursive(&source, &destination).unwrap();

        assert_eq!(
            fs::read_to_string(destination.join("top.txt")).unwrap(),
            "top"
        );
        assert_eq!(
            fs::read_to_string(destination.join("sub/nested.txt")).unwrap(),
            "nested"
        );
    }

    #[cfg(unix)]
    #[test]
    fn copy_dir_recursive_skips_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let destination = dir.path().join("destination");

        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("real.txt"), "real").unwrap();
        std::os::unix::fs::symlink(source.join("real.txt"), source.join("link.txt")).unwrap();
        fs::create_dir_all(&destination).unwrap();

        copy_dir_recursive(&source, &destination).unwrap();

        assert!(destination.join("real.txt").exists());
        assert!(!destination.join("link.txt").exists());
    }

    #[test]
    fn sync_file_copies_plain_file() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("nested/destination.txt");
        fs::write(&source, "content").unwrap();

        let note = sync_file(&source, &destination).unwrap();

        assert!(note.is_none());
        assert_eq!(fs::read_to_string(&destination).unwrap(), "content");
    }

    #[test]
    fn sync_file_errors_when_source_missing() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("missing.txt");
        let destination = dir.path().join("destination.txt");

        let err = sync_file(&source, &destination).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[cfg(unix)]
    #[test]
    fn sync_file_converts_self_referencing_symlink_to_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("destination.txt");
        fs::write(&destination, "hello").unwrap();

        let source = dir.path().join("source.txt");
        std::os::unix::fs::symlink(destination.canonicalize().unwrap(), &source).unwrap();

        let note = sync_file(&source, &destination).unwrap();

        assert!(note.is_some());
        assert!(!source.is_symlink());
        assert_eq!(fs::read_to_string(&source).unwrap(), "hello");
        assert_eq!(fs::read_to_string(&destination).unwrap(), "hello");
    }

    #[test]
    fn sync_directory_contents_prunes_stale_entries() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let destination = dir.path().join("destination");

        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("keep.txt"), "keep").unwrap();

        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("stale.txt"), "stale").unwrap();

        sync_directory_contents(&source, &destination).unwrap();

        assert!(destination.join("keep.txt").exists());
        assert!(!destination.join("stale.txt").exists());
    }

    #[test]
    fn sync_directory_contents_prunes_stale_subdirectories() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let destination = dir.path().join("destination");

        fs::create_dir_all(&source).unwrap();

        fs::create_dir_all(destination.join("stale_dir")).unwrap();
        fs::write(destination.join("stale_dir/f.txt"), "stale").unwrap();

        sync_directory_contents(&source, &destination).unwrap();

        assert!(!destination.join("stale_dir").exists());
    }

    #[test]
    fn sync_directory_copies_nested_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let destination = dir.path().join("destination");

        fs::create_dir_all(source.join("sub")).unwrap();
        fs::write(source.join("top.txt"), "top").unwrap();
        fs::write(source.join("sub/nested.txt"), "nested").unwrap();

        let note = sync_directory(&source, &destination).unwrap();

        assert!(note.is_none());
        assert_eq!(
            fs::read_to_string(destination.join("top.txt")).unwrap(),
            "top"
        );
        assert_eq!(
            fs::read_to_string(destination.join("sub/nested.txt")).unwrap(),
            "nested"
        );
    }

    #[cfg(unix)]
    #[test]
    fn sync_directory_converts_self_referencing_symlink_to_real_directory() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("destination");
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("f.txt"), "hello").unwrap();

        let source = dir.path().join("source");
        std::os::unix::fs::symlink(destination.canonicalize().unwrap(), &source).unwrap();

        let note = sync_directory(&source, &destination).unwrap();

        assert!(note.is_some());
        assert!(!source.is_symlink());
        assert!(source.is_dir());
        assert_eq!(fs::read_to_string(source.join("f.txt")).unwrap(), "hello");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_self_reference_detects_symlink_pointing_at_destination() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("destination");
        fs::create_dir_all(&destination).unwrap();

        let source = dir.path().join("source");
        std::os::unix::fs::symlink(destination.canonicalize().unwrap(), &source).unwrap();

        let detected = symlink_self_reference(&source, &destination).unwrap();

        assert_eq!(detected, destination.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_self_reference_ignores_symlink_pointing_elsewhere() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("destination");
        let elsewhere = dir.path().join("elsewhere");
        fs::create_dir_all(&destination).unwrap();
        fs::create_dir_all(&elsewhere).unwrap();

        let source = dir.path().join("source");
        std::os::unix::fs::symlink(&elsewhere, &source).unwrap();

        assert!(symlink_self_reference(&source, &destination).is_none());
    }

    #[test]
    fn symlink_self_reference_ignores_non_symlink_source() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("destination");
        let source = dir.path().join("source");
        fs::create_dir_all(&destination).unwrap();
        fs::create_dir_all(&source).unwrap();

        assert!(symlink_self_reference(&source, &destination).is_none());
    }
}
