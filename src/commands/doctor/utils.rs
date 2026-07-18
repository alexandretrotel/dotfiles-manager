use crate::commands::doctor::types::ValidationError;
use crate::profiles::ActiveProfile;
use crate::registry::config::ConfigRegistry;
use crate::utils::paths::get_config_registry_path;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Resolve every enabled `.json` config entry to its on-disk path for the given
/// profile. Shared by the JSON validator and `doctor fix` so both walk the same
/// set of files.
pub fn enabled_json_files(profile: &ActiveProfile) -> eyre::Result<Vec<(PathBuf, String)>> {
    let config_registry_path = get_config_registry_path();
    let config_registry = ConfigRegistry::load_or_create(&config_registry_path)?;

    let mut files = Vec::new();
    for (_id, entry) in config_registry.get_enabled_entries() {
        if entry.source_path.ends_with(".json")
            && let Some(resolved) = profile.resolve_source(&entry.source_path)
        {
            files.push((resolved.path, entry.name.clone()));
        }
    }
    Ok(files)
}

pub fn validate_json_file(path: &Path, description: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if !path.exists() {
        return errors;
    }
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            errors.push(
                ValidationError::warning(format!("Could not read {}: {}", description, e))
                    .with_fix(format!("Check file permissions for {}", path.display())),
            );
            return errors;
        }
    };
    if let Err(e) = serde_json::from_str::<serde_json::Value>(&content) {
        errors.push(
            ValidationError::error(format!("Invalid JSON in {}: {}", description, e))
                .with_fix(format!("Check syntax in {}", path.display())),
        );
    }
    errors
}

pub fn create_temp_file_path() -> std::io::Result<std::path::PathBuf> {
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    let base = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    for attempt in 0..10 {
        let name = format!("mntn-doctor-{}-{}-{}", pid, base, attempt);
        let path = dir.join(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => return Ok(path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }

    Err(std::io::Error::other("Failed to create temporary file"))
}
