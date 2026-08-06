use std::{collections::HashSet, ffi::OsString, fs, io, path::Path, path::PathBuf};

/// Recursively copies `src` dir into `dst`, skipping symlinks.
pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    copy_dir(src, dst, None)
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

/// Mirrors `source` contents into `dest`, deleting dest entries absent from source.
///
/// Recurses into shared subdirectories so stale files nested arbitrarily deep are
/// also removed.
pub(crate) fn sync_directory_contents(source: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    let mut seen = HashSet::new();
    copy_dir(source, dest, Some(&mut seen))?;
    prune_extraneous(dest, &seen)
}

/// Copies `src` into `dst`, skipping symlinks. When `seen` is given, every
/// copied entry's name is recorded into it and subdirectories are mirrored
/// (stale entries pruned) rather than plain-copied.
fn copy_dir(src: &Path, dst: &Path, mut seen: Option<&mut HashSet<OsString>>) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let src_path = entry.path();
        let dst_path = dst.join(&name);

        let metadata = fs::symlink_metadata(&src_path)?;
        if metadata.file_type().is_symlink() {
            continue;
        }

        if let Some(seen) = seen.as_deref_mut() {
            seen.insert(name);
        }

        if metadata.is_dir() {
            fs::create_dir_all(&dst_path)?;
            match seen.as_deref_mut() {
                Some(_) => sync_directory_contents(&src_path, &dst_path)?,
                None => copy_dir(&src_path, &dst_path, None)?,
            }
        } else if metadata.is_file() {
            fs::copy(&src_path, &dst_path)?;
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
