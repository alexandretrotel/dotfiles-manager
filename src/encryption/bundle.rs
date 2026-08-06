use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

use crate::error::{Result, WrapErr};

/// Normalize a tar entry path to forward slashes with no leading `./`, so
/// paths read back out match the `source_path` strings used as archive
/// member names.
pub(crate) fn normalize_tar_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

/// Write a deterministic tar archive with the given `(archive path, file)`
/// entries.
pub fn write_entries_tar(tar_path: &Path, entries: &[(&str, &Path)]) -> Result<()> {
    let file = fs::File::create(tar_path)
        .wrap_err_with(|| format!("Create tar archive {}", tar_path.display()))?;
    let mut builder = tar::Builder::new(file);
    builder.mode(tar::HeaderMode::Deterministic);
    for (source_path, target_path) in entries {
        let mut input = fs::File::open(target_path).wrap_err_with(|| {
            format!(
                "Open {} for archiving as {}",
                target_path.display(),
                source_path
            )
        })?;
        builder
            .append_file(*source_path, &mut input)
            .wrap_err_with(|| format!("Append {} to tar", source_path))?;
    }
    builder.finish().wrap_err("Finish tar archive")?;
    Ok(())
}

/// Recursively list every regular file under `dir`, paired with the tar
/// member name it should be archived under (`{source_prefix}/relative/path`,
/// forward slashes regardless of platform). Symlinks are skipped.
pub fn collect_dir_tar_entries(source_prefix: &str, dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut entries = Vec::new();
    collect_dir_tar_entries_into(source_prefix, dir, &mut entries)
        .wrap_err_with(|| format!("Walk directory {}", dir.display()))?;
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

fn collect_dir_tar_entries_into(
    prefix: &str,
    dir: &Path,
    entries: &mut Vec<(String, PathBuf)>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let member = format!("{prefix}/{}", entry.file_name().to_string_lossy());

        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            continue;
        } else if metadata.is_dir() {
            collect_dir_tar_entries_into(&member, &path, entries)?;
        } else if metadata.is_file() {
            entries.push((member, path));
        }
    }
    Ok(())
}

/// Read every regular file in a tar archive into memory, keyed by archive
/// path.
pub fn load_tar_member_map(tar_path: &Path) -> Result<HashMap<String, Vec<u8>>> {
    let file = fs::File::open(tar_path).wrap_err("Open tar for reading")?;
    let mut archive = tar::Archive::new(file);
    let mut map = HashMap::new();
    for entry in archive.entries().wrap_err("Read tar entries")? {
        let mut entry = entry.wrap_err("Tar entry")?;
        if entry.header().entry_type() != tar::EntryType::Regular {
            continue;
        }
        let path = normalize_tar_path(&entry.path().wrap_err("Tar entry path")?);
        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .wrap_err_with(|| format!("Read tar member {}", path))?;
        map.insert(path, data);
    }
    Ok(map)
}

/// Make a file private (mode 0600). No-op on non-unix platforms.
pub fn set_private_file_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .wrap_err_with(|| format!("Set permissions on: {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// Create a unique temp file that is removed when the returned handle is
/// dropped.
pub fn create_temp_file(label: &str) -> std::io::Result<NamedTempFile> {
    tempfile::Builder::new()
        .prefix(&format!("dfm-{label}-"))
        .tempfile()
}
