use crate::commands::core::Command;
use crate::commands::doctor::utils::enabled_json_files;
use crate::profiles::ActiveProfile;
use crate::utils::display::{green, red, yellow};
use std::fs;
use std::path::Path;

pub(crate) struct FixTask {
    profile: ActiveProfile,
    dry_run: bool,
}

impl FixTask {
    pub(crate) fn new(profile: ActiveProfile, dry_run: bool) -> Self {
        Self { profile, dry_run }
    }
}

/// Outcome of processing a single JSON file.
enum FixOutcome {
    /// File was already pretty-printed; nothing to do.
    Unchanged,
    /// File was (or would be) reformatted.
    Reformatted,
    /// File could not be repaired (unreadable, true syntax error, or write failure).
    Unfixable(String),
}

impl Command for FixTask {
    fn name(&self) -> &str {
        "Doctor fix"
    }

    fn execute(&mut self) -> eyre::Result<()> {
        if self.dry_run {
            println!("Reformatting JSON configs (dry run)...");
        } else {
            println!("Reformatting JSON configs...");
        }
        println!("   Profile: {}", self.profile);

        let files = enabled_json_files(&self.profile)?;
        if files.is_empty() {
            println!("{}", green("No JSON config files to format"));
            return Ok(());
        }

        let mut reformatted = 0usize;
        let mut unchanged = 0usize;
        let mut unfixable = 0usize;

        for (path, name) in files {
            match self.fix_file(&path) {
                FixOutcome::Unchanged => {
                    unchanged += 1;
                }
                FixOutcome::Reformatted => {
                    reformatted += 1;
                    let verb = if self.dry_run {
                        "Would reformat"
                    } else {
                        "Reformatted"
                    };
                    println!(
                        "{}",
                        green(&format!(" {} {} ({})", verb, name, path.display()))
                    );
                }
                FixOutcome::Unfixable(reason) => {
                    unfixable += 1;
                    println!("{}", red(&format!(" x {}: {}", name, reason)));
                    println!(
                        "{}",
                        yellow(&format!(
                            " Fix: repair the syntax manually in {}",
                            path.display()
                        ))
                    );
                }
            }
        }

        println!();
        println!(
            "{} reformatted, {} already clean, {} unfixable",
            reformatted, unchanged, unfixable
        );

        if unfixable > 0 {
            return Err(eyre::eyre!(
                "{} file(s) have syntax errors serde_json cannot repair",
                unfixable
            ));
        }

        Ok(())
    }
}

impl FixTask {
    fn fix_file(&self, path: &Path) -> FixOutcome {
        if !path.exists() {
            return FixOutcome::Unchanged;
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return FixOutcome::Unfixable(format!("could not read file: {}", e)),
        };

        let value: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => return FixOutcome::Unfixable(format!("invalid JSON: {}", e)),
        };

        let mut pretty = match serde_json::to_string_pretty(&value) {
            Ok(p) => p,
            Err(e) => return FixOutcome::Unfixable(format!("could not serialize: {}", e)),
        };
        pretty.push('\n');

        if pretty == content {
            return FixOutcome::Unchanged;
        }

        if self.dry_run {
            return FixOutcome::Reformatted;
        }

        if let Err(e) = write_atomic(path, &pretty) {
            return FixOutcome::Unfixable(format!("could not write file: {}", e));
        }

        FixOutcome::Reformatted
    }
}

/// Write `content` to `path` atomically by writing a sibling temp file and
/// renaming it into place, so a crash mid-write cannot truncate the original.
fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    let tmp_path = path.with_extension("json.mntn-tmp");
    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, path)
}
