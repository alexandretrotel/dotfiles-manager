use std::fs;
use std::path::Path;

use super::fs_ops::{backup_directory, backup_file};
use crate::context::Dotfm;
use crate::error::{Result, WrapErr};
use crate::registry::ConfigRegistry;
use crate::report::{ItemOutcome, SectionReport};

/// Copy every enabled config registry entry into `configs_path`.
pub(super) fn backup_configs(ctx: &Dotfm, configs_path: &Path) -> Result<SectionReport> {
    let config_registry_path = ctx.config_registry_path();
    let config_registry = ConfigRegistry::load_or_create(&config_registry_path)
        .wrap_err_with(|| format!("Load config registry: {}", config_registry_path.display()))?;

    let enabled_entries: Vec<_> = config_registry.get_enabled_entries().collect();

    let mut report = SectionReport::default();

    for (id, entry) in enabled_entries {
        let target_path = &entry.target_path;
        let backup_destination = configs_path.join(&entry.source_path);

        let entry_result: Result<Option<String>> = (|| {
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

        report.outcomes.push(match entry_result {
            Ok(Some(note)) => ItemOutcome::done_with_note(id, &entry.source_path, note),
            Ok(None) => ItemOutcome::done(id, &entry.source_path),
            Err(e) => ItemOutcome::skipped(id, &entry.source_path, e.to_string()),
        });
    }

    Ok(report)
}
