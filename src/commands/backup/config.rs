use super::utils::{backup_directory, backup_file};
use crate::registry::config::ConfigRegistry;
use crate::utils::display::{green, yellow};
use crate::utils::paths::get_config_registry_path;
use eyre::{Result, WrapErr};
use std::fs;
use std::path::Path;

pub fn backup_configs(configs_path: &Path) -> Result<(u32, u32)> {
    let config_registry_path = get_config_registry_path();
    let config_registry = ConfigRegistry::load_or_create(&config_registry_path)
        .wrap_err_with(|| format!("Load config registry: {}", config_registry_path.display()))?;

    let enabled_entries: Vec<_> = config_registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        println!("No configuration files found to backup");
        return Ok((0, 0));
    }

    println!("   Configurations: {} entries", enabled_entries.len());

    let mut succeeded = 0;
    let mut skipped = 0;

    for (id, entry) in enabled_entries {
        let target_path = &entry.target_path;
        let backup_destination = configs_path.join(&entry.source_path);

        let entry_result: Result<()> = (|| {
            if let Some(parent) = backup_destination.parent() {
                fs::create_dir_all(parent).wrap_err_with(|| {
                    format!("Prepare backup path {} ({})", parent.display(), id)
                })?;
            }

            if target_path.is_dir() {
                backup_directory(target_path, &backup_destination).wrap_err_with(|| {
                    format!(
                        "Copy directory {} -> {}",
                        target_path.display(),
                        backup_destination.display()
                    )
                })
            } else {
                backup_file(target_path, &backup_destination).wrap_err_with(|| {
                    format!(
                        "Copy file {} -> {}",
                        target_path.display(),
                        backup_destination.display()
                    )
                })
            }
        })();

        match entry_result {
            Ok(()) => {
                succeeded += 1;
                println!("     {} {}", green("✔"), entry.source_path);
            }
            Err(e) => {
                skipped += 1;
                eprintln!(
                    "{}",
                    yellow(&format!(
                        "     skipped {} ({}): {}",
                        entry.source_path, id, e
                    ))
                );
            }
        }
    }

    Ok((succeeded, skipped))
}
