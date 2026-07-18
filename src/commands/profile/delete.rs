use crate::profiles::{ProfileConfig, get_active_profile_name};
use crate::utils::paths::{get_profiles_config_path, get_profiles_path};
use eyre::{Result, WrapErr, bail};

pub(crate) fn delete_profile(name: &str) -> Result<()> {
    let path = get_profiles_config_path();
    let mut config = ProfileConfig::load_or_default();

    if !config.profile_exists(name) {
        bail!("Profile '{}' does not exist", name);
    }

    if let Some(current) = get_active_profile_name()
        && current == name
    {
        bail!(
            "Cannot delete active profile '{}'. Switch to another profile first.",
            name
        );
    }

    config.delete_profile(name);
    config
        .save(&path)
        .wrap_err_with(|| format!("Save profile config to {}", path.display()))?;

    let profile_dir = get_profiles_path(name);
    if profile_dir.exists() {
        println!("Profile directory exists at {}", profile_dir.display());
        println!("The directory was NOT deleted. Remove manually if desired:");
        println!("rm -rf {}", profile_dir.display());
    }

    println!("Deleted profile '{}'", name);
    Ok(())
}
