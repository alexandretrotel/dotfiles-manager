use crate::profiles::ProfileConfig;
use crate::utils::paths::{get_profiles_config_path, get_profiles_path};
use eyre::{Result, WrapErr, bail};
use std::fs;

pub(crate) fn create_profile(name: &str, description: Option<String>) -> Result<()> {
    let path = get_profiles_config_path();
    let mut config = ProfileConfig::load_or_default();

    if config.profile_exists(name) {
        bail!("Profile '{}' already exists", name);
    }

    if name.is_empty() {
        bail!("Profile name cannot be empty");
    }

    if name
        .chars()
        .any(|c| !c.is_alphanumeric() && c != '-' && c != '_')
    {
        bail!("Profile name can only contain letters, numbers, hyphens, and underscores");
    }

    config.create_profile(name, description.clone());
    if config.version.is_empty() {
        config.version = "1.0.0".to_string();
    }

    config
        .save(&path)
        .wrap_err_with(|| format!("Save profile config to {}", path.display()))?;

    let profile_dir = get_profiles_path(name);
    fs::create_dir_all(&profile_dir).wrap_err_with(|| {
        format!(
            "Create profile directory at {} (config was saved)",
            profile_dir.display()
        )
    })?;

    println!("Created profile '{}'", name);
    if let Some(desc) = description {
        println!("   Description: {}", desc);
    }
    println!();
    println!("Switch to this profile with: mntn use {}", name);
    Ok(())
}
