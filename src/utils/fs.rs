use std::{fs, io, path::Path};

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
