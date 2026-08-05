use std::{fs, io, path::Path, process::Command};

/// Recursively copies `src` dir into `dst`, skipping symlinks.
pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        let metadata = fs::symlink_metadata(&src_path)?;
        if metadata.file_type().is_symlink() {
            continue;
        } else if metadata.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if metadata.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Mirrors `source` contents into `dest` via rsync, deleting extraneous dest files.
pub(crate) fn sync_directory_contents(source: &Path, dest: &Path) -> io::Result<()> {
    let output = Command::new("rsync")
        .args(["-av", "--delete"])
        .arg(format!("{}/", source.display()))
        .arg(dest)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr.clone())
            .unwrap_or_else(|_| format!("<binary stderr: {} bytes>", output.stderr.len()));
        return Err(io::Error::other(format!("rsync failed: {}", stderr)));
    }

    Ok(())
}
