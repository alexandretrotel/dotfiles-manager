use std::{collections::HashSet, ffi::OsString, fs, io, path::Path};

/// Recursively copies `src` dir into `dst`, skipping symlinks.
pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    copy_dir(src, dst, None)
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
