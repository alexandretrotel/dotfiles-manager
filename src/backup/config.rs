use std::fs;
use std::path::Path;

use crate::context::Dfm;
use crate::error::{Result, WrapErr};
use crate::registry::ConfigRegistry;
use crate::report::{ItemOutcome, SectionReport};
use crate::utils::fs::{sync_directory, sync_file};

/// Copy every enabled config registry entry into `configs_path`.
pub(super) fn backup_configs(ctx: &Dfm, configs_path: &Path) -> Result<SectionReport> {
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
                sync_directory(target_path, &backup_destination).wrap_err_with(|| {
                    format!(
                        "Copy directory {} -> {}",
                        target_path.display(),
                        backup_destination.display()
                    )
                })
            } else {
                sync_file(target_path, &backup_destination).wrap_err_with(|| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ConfigRegistryEntry;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn entry(source_path: &str, target_path: PathBuf) -> ConfigRegistryEntry {
        ConfigRegistryEntry {
            name: source_path.to_string(),
            description: None,
            enabled: true,
            source_path: source_path.to_string(),
            target_path,
        }
    }

    fn save_registry(ctx: &Dfm, entries: HashMap<String, ConfigRegistryEntry>) {
        let registry = ConfigRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.config_registry_path()).unwrap();
    }

    #[test]
    fn backs_up_a_plain_file_entry() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let target = dir.path().join("source.txt");
        fs::write(&target, b"hello").unwrap();

        let mut entries = HashMap::new();
        entries.insert("file".to_string(), entry("file.txt", target.clone()));
        save_registry(&ctx, entries);

        let configs_path = dir.path().join("configs");
        fs::create_dir_all(&configs_path).unwrap();

        let report = backup_configs(&ctx, &configs_path).unwrap();

        assert_eq!(report.succeeded(), 1);
        assert_eq!(fs::read(configs_path.join("file.txt")).unwrap(), b"hello");
    }

    #[test]
    fn backs_up_a_directory_entry_with_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let target = dir.path().join("source_dir");
        fs::create_dir_all(target.join("nested")).unwrap();
        fs::write(target.join("top.txt"), b"top").unwrap();
        fs::write(target.join("nested/inner.txt"), b"inner").unwrap();

        let mut entries = HashMap::new();
        entries.insert("dir".to_string(), entry("mydir", target.clone()));
        save_registry(&ctx, entries);

        let configs_path = dir.path().join("configs");
        fs::create_dir_all(&configs_path).unwrap();

        let report = backup_configs(&ctx, &configs_path).unwrap();

        assert_eq!(report.succeeded(), 1);
        assert_eq!(
            fs::read(configs_path.join("mydir/top.txt")).unwrap(),
            b"top"
        );
        assert_eq!(
            fs::read(configs_path.join("mydir/nested/inner.txt")).unwrap(),
            b"inner"
        );
    }

    #[test]
    fn missing_target_path_is_skipped_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));
        let missing_target = dir.path().join("does-not-exist.txt");

        let mut entries = HashMap::new();
        entries.insert("missing".to_string(), entry("missing.txt", missing_target));
        save_registry(&ctx, entries);

        let configs_path = dir.path().join("configs");
        fs::create_dir_all(&configs_path).unwrap();

        let report = backup_configs(&ctx, &configs_path).unwrap();

        assert_eq!(report.succeeded(), 0);
        assert_eq!(report.skipped(), 1);
        assert!(!configs_path.join("missing.txt").exists());
    }
}
