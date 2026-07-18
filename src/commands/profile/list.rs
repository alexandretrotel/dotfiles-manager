use crate::profiles::{ProfileConfig, get_active_profile_name};
use eyre::Result;

pub(crate) fn list_profiles() -> Result<()> {
    let config = ProfileConfig::load_or_default();
    let profiles = config.list_profiles();
    let current = get_active_profile_name();

    if profiles.is_empty() {
        println!("No profiles configured");
        println!();
        println!("Create a profile with: mntn profile create <name>");
        return Ok(());
    }

    println!("Available profiles:");
    for name in profiles {
        let is_current = current.as_ref() == Some(name);
        let marker = if is_current { " ← active" } else { "" };

        if let Some(def) = config.get_profile(name) {
            if let Some(desc) = &def.description {
                println!("   {} - {}{}", name, desc, marker);
            } else {
                println!("   {}{}", name, marker);
            }
        } else {
            println!("   {}{}", name, marker);
        }
    }
    Ok(())
}
