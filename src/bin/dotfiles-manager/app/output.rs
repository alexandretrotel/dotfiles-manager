use anstream::{eprintln, println};
use anstyle::{AnsiColor, Style};
use dotfiles_manager::doctor::{DoctorReport, FixedFile, Severity};
use dotfiles_manager::{RegistryEntryStatus, SectionReport};

/// Style used to render success/positive output in green.
const GREEN: Style = AnsiColor::Green.on_default();
/// Style used to render warning output in yellow.
const YELLOW: Style = AnsiColor::Yellow.on_default();
/// Style used to render error output in red.
const RED: Style = AnsiColor::Red.on_default();

/// Wrap `text` in the given style, resetting after it. Rendered through
/// `anstream`, so the escape codes degrade gracefully on non-color
/// terminals, `NO_COLOR`, and legacy Windows consoles.
fn color(text: &str, style: Style) -> String {
    format!("{style}{text}{style:#}")
}

/// Wrap `text` in green (success).
pub fn green(text: &str) -> String {
    color(text, GREEN)
}

/// Wrap `text` in yellow (warning).
pub fn yellow(text: &str) -> String {
    color(text, YELLOW)
}

/// Wrap `text` in red (error).
pub fn red(text: &str) -> String {
    color(text, RED)
}

/// Print a backup/restore section: warnings first, then one line per entry.
fn print_section(title: &str, section: &SectionReport) {
    println!("   {}: {} entries", title, section.outcomes.len());

    for warning in &section.warnings {
        eprintln!("{}", yellow(&format!("     {}", warning)));
    }

    for outcome in &section.outcomes {
        match &outcome.status {
            RegistryEntryStatus::Done { note } => {
                println!("     {} {}", green("✔"), outcome.label);
                if let Some(note) = note {
                    println!("       {}", note);
                }
            }
            RegistryEntryStatus::Skipped { reason } => {
                eprintln!(
                    "{}",
                    yellow(&format!(
                        "     skipped {} ({}): {}",
                        outcome.label, outcome.id, reason
                    ))
                );
            }
        }
    }
}

/// Print a section's per-entry lines followed by its succeeded/skipped
/// summary.
pub fn print_section_with_summary(title: &str, section: &SectionReport) {
    print_section(title, section);
    print_section_summary(title, section);
}

/// Print a section's succeeded/skipped counts.
fn print_section_summary(title: &str, section: &SectionReport) {
    println!(
        "   {} completed: {} succeeded, {} skipped",
        title,
        section.succeeded(),
        section.skipped()
    );
}

/// Print every validator's findings, grouped and severity-colored.
pub fn print_doctor_report(report: &DoctorReport) {
    for (name, errors) in report.results() {
        if errors.is_empty() {
            println!(" {} OK", name);
        } else {
            println!(" {}", name);
            for error in errors {
                let line = match error.severity {
                    Severity::Error => red(&format!(" x {}", error.message)),
                    Severity::Warning => yellow(&format!(" ! {}", error.message)),
                    Severity::Info => green(&format!(" i {}", error.message)),
                };
                println!("{}", line);
                if let Some(fix) = &error.fix_suggestion {
                    println!("{}", yellow(&format!(" Fix: {}", fix)));
                }
            }
        }
    }
}

/// Print the outcome of `dfm doctor --fix` rewriting each of dfm's own
/// registry/config files.
pub fn print_fix_report(files: &[FixedFile]) {
    println!(" Fix");
    for file in files {
        match &file.outcome {
            Ok(true) => println!(
                "{}",
                green(&format!(
                    " ✔ Rewrote {} ({})",
                    file.label,
                    file.path.display()
                ))
            ),
            Ok(false) => println!(" - {} does not exist, skipped", file.label),
            Err(e) => println!(
                "{}",
                red(&format!(
                    " x Could not rewrite {} ({}): {}",
                    file.label,
                    file.path.display(),
                    e
                ))
            ),
        }
    }
}

// `print_section`, `print_doctor_report`, and `print_fix_report` write
// directly to stdout/stderr via `anstream`'s `println!`/`eprintln!`, so
// meaningfully asserting on their output would require capturing those
// streams, which isn't worth the effort here. Only the pure color helpers
// below are tested.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_contains_original_text() {
        assert!(green("hello").contains("hello"));
    }

    #[test]
    fn yellow_contains_original_text() {
        assert!(yellow("warning").contains("warning"));
    }

    #[test]
    fn red_contains_original_text() {
        assert!(red("error").contains("error"));
    }

    #[test]
    fn color_contains_original_text() {
        assert!(color("some text", GREEN).contains("some text"));
    }

    #[test]
    fn color_wraps_text_with_style_and_reset() {
        let styled = color("x", RED);
        let expected = format!("{RED}x{RED:#}");
        assert_eq!(styled, expected);
    }

    #[test]
    fn different_colors_produce_different_output_for_same_text() {
        assert_ne!(green("same"), yellow("same"));
        assert_ne!(yellow("same"), red("same"));
        assert_ne!(green("same"), red("same"));
    }

    #[test]
    fn empty_text_still_contains_empty_substring() {
        assert!(green("").contains(""));
    }
}
