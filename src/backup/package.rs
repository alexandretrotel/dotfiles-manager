use std::fs;
use std::io::Write;
use std::path::Path;
use std::thread;

use crate::context::Dfm;
use crate::error::{Result, WrapErr};
use crate::registry::{PackageRegistry, PackageRegistryEntry};
use crate::report::{ItemOutcome, SectionReport};
use crate::utils::ansi::strip_ansi_codes;
use crate::utils::process::run_cmd;

/// Run every platform-compatible, enabled package manager export command in
/// parallel and write its output into `packages_path`.
pub(super) fn backup_packages(ctx: &Dfm, packages_path: &Path) -> Result<SectionReport> {
    let package_registry_path = ctx.package_registry_path();
    let package_registry = PackageRegistry::load_or_create(&package_registry_path)
        .wrap_err_with(|| format!("Load package registry: {}", package_registry_path.display()))?;

    let compatible_entries: Vec<_> = package_registry
        .get_platform_compatible_entries(std::env::consts::OS)
        .collect();

    let mut outcomes: Vec<PackageBackupOutcome> = thread::scope(|s| {
        let mut handles = Vec::with_capacity(compatible_entries.len());
        for (id, entry) in compatible_entries {
            let id = id.clone();
            let entry = entry.clone();
            handles.push(s.spawn(|| run_single_package_backup(packages_path, id, entry)));
        }
        handles
            .into_iter()
            .map(|h| h.join().expect("package backup thread panicked"))
            .collect()
    });

    outcomes.sort_by(|a, b| a.output_file.cmp(&b.output_file));

    let mut report = SectionReport::default();
    for o in outcomes {
        report.outcomes.push(match o.result {
            Ok(()) => ItemOutcome::done(o.id, o.output_file),
            Err(e) => ItemOutcome::skipped(o.id, o.output_file, e.to_string()),
        });
    }

    Ok(report)
}

/// Result of exporting one package manager's installed-package list.
struct PackageBackupOutcome {
    id: String,
    output_file: String,
    result: Result<()>,
}

/// Run one package manager's export command and write its (ANSI-stripped)
/// output to `packages_path` via a temp file + rename.
fn run_single_package_backup(
    packages_path: &Path,
    id: String,
    entry: PackageRegistryEntry,
) -> PackageBackupOutcome {
    let output_file = entry.output_file.clone();

    let result: Result<()> = (|| {
        let args: Vec<&str> = entry.args.iter().map(|s| s.as_str()).collect();
        let content = run_cmd(&entry.command, &args, None)
            .wrap_err_with(|| format!("Command {} failed for {}", entry.command, id))?;

        let content = strip_ansi_codes(&content);
        let output_path = packages_path.join(&entry.output_file);
        let tmp_path = output_path.with_extension("tmp");

        let mut tmp_file = fs::File::create(&tmp_path)
            .wrap_err_with(|| format!("Create temp file for {}", entry.output_file))?;
        tmp_file
            .write_all(content.as_bytes())
            .wrap_err_with(|| format!("Write temp file for {}", entry.output_file))?;

        fs::rename(&tmp_path, &output_path)
            .wrap_err_with(|| format!("Move {} into place", entry.output_file))?;
        Ok(())
    })();

    PackageBackupOutcome {
        id,
        output_file,
        result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn entry(command: &str, args: Vec<&str>, output_file: &str) -> PackageRegistryEntry {
        PackageRegistryEntry {
            name: command.to_string(),
            description: None,
            enabled: true,
            command: command.to_string(),
            args: args.into_iter().map(String::from).collect(),
            output_file: output_file.to_string(),
            platforms: None,
        }
    }

    fn save_registry(ctx: &Dfm, entries: HashMap<String, PackageRegistryEntry>) {
        let registry = PackageRegistry {
            version: "1.0.0".to_string(),
            entries,
        };
        registry.save(&ctx.package_registry_path()).unwrap();
    }

    #[test]
    fn writes_exported_package_list_to_output_file() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));

        let mut entries = HashMap::new();
        entries.insert(
            "fake".to_string(),
            entry("echo", vec!["package-a\npackage-b"], "fake.txt"),
        );
        save_registry(&ctx, entries);

        let packages_path = dir.path().join("packages");
        fs::create_dir_all(&packages_path).unwrap();

        let report = backup_packages(&ctx, &packages_path).unwrap();

        assert_eq!(report.succeeded(), 1);
        let content = fs::read_to_string(packages_path.join("fake.txt")).unwrap();
        assert_eq!(content, "package-a\npackage-b\n");
    }

    #[test]
    fn missing_package_manager_command_is_skipped_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));

        let mut entries = HashMap::new();
        entries.insert(
            "ghost".to_string(),
            entry("definitely-not-a-real-command-xyz", vec![], "ghost.txt"),
        );
        save_registry(&ctx, entries);

        let packages_path = dir.path().join("packages");
        fs::create_dir_all(&packages_path).unwrap();

        let report = backup_packages(&ctx, &packages_path).unwrap();

        assert_eq!(report.succeeded(), 0);
        assert_eq!(report.skipped(), 1);
        assert!(!packages_path.join("ghost.txt").exists());
    }

    #[test]
    fn platform_incompatible_entries_are_excluded_from_report() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path().join("dfm"));

        let mut entries = HashMap::new();
        entries.insert(
            "wrong_platform".to_string(),
            PackageRegistryEntry {
                platforms: Some(vec!["definitely-not-this-os".to_string()]),
                ..entry("echo", vec!["hi"], "wrong.txt")
            },
        );
        save_registry(&ctx, entries);

        let packages_path = dir.path().join("packages");
        fs::create_dir_all(&packages_path).unwrap();

        let report = backup_packages(&ctx, &packages_path).unwrap();

        assert!(report.outcomes.is_empty());
    }
}
