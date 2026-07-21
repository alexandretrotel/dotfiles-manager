use crate::registry::package::{PackageRegistry, PackageRegistryEntry};
use crate::utils::display::{green, yellow};
use crate::utils::paths::get_package_registry_path;
use crate::utils::system::{run_cmd, strip_ansi_codes};
use color_eyre::eyre::{Result, WrapErr};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::thread;

pub fn backup_packages(packages_path: &Path) -> Result<(u32, u32)> {
    let package_registry_path = get_package_registry_path();
    let package_registry = PackageRegistry::load_or_create(&package_registry_path)
        .wrap_err_with(|| format!("Load package registry: {}", package_registry_path.display()))?;

    let current_platform = PackageRegistry::get_current_platform();
    let compatible_entries: Vec<_> = package_registry
        .get_platform_compatible_entries(&current_platform)
        .collect();

    if compatible_entries.is_empty() {
        println!("No package managers found to backup");
        return Ok((0, 0));
    }

    println!("   Package managers: {} entries", compatible_entries.len());

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

    let mut succeeded = 0;
    let mut skipped = 0;

    for o in outcomes {
        match o.result {
            Ok(()) => {
                succeeded += 1;
                println!("     {} {}", green("✔"), o.output_file);
            }
            Err(e) => {
                skipped += 1;
                eprintln!(
                    "{}",
                    yellow(&format!("     skipped {} ({}): {}", o.output_file, o.id, e))
                );
            }
        }
    }

    Ok((succeeded, skipped))
}

struct PackageBackupOutcome {
    id: String,
    output_file: String,
    result: Result<()>,
}

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
