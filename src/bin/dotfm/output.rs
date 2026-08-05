use dotfm::doctor::{DoctorFixOutcome, DoctorFixReport, DoctorReport, Severity};
use dotfm::{ItemStatus, SectionReport};

const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_YELLOW: &str = "\x1b[33m";
const COLOR_RED: &str = "\x1b[31m";
const COLOR_RESET: &str = "\x1b[0m";

/// Wrap `text` in the given ANSI color code, resetting after it.
fn color(text: &str, code: &str) -> String {
    format!("{code}{text}{COLOR_RESET}")
}

/// Wrap `text` in green (success).
pub fn green(text: &str) -> String {
    color(text, COLOR_GREEN)
}

/// Wrap `text` in yellow (warning).
pub fn yellow(text: &str) -> String {
    color(text, COLOR_YELLOW)
}

/// Wrap `text` in red (error).
pub fn red(text: &str) -> String {
    color(text, COLOR_RED)
}

/// Print a backup/restore section: warnings first, then one line per item.
fn print_section(title: &str, section: &SectionReport) {
    println!("   {}: {} entries", title, section.outcomes.len());

    for warning in &section.warnings {
        eprintln!("{}", yellow(&format!("     {}", warning)));
    }

    for outcome in &section.outcomes {
        match &outcome.status {
            ItemStatus::Done { note } => {
                println!("     {} {}", green("✔"), outcome.label);
                if let Some(note) = note {
                    println!("       {}", note);
                }
            }
            ItemStatus::Skipped { reason } => {
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

/// Print a section's per-item lines followed by its succeeded/skipped
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

/// Print each fixed file's outcome, then a reformatted/unchanged/unfixable
/// summary line.
pub fn print_doctor_fix_report(report: &DoctorFixReport) {
    for entry in &report.entries {
        match &entry.outcome {
            DoctorFixOutcome::Unchanged => {}
            DoctorFixOutcome::Reformatted => {
                println!(
                    "{}",
                    green(&format!(
                        " Reformatted {} ({})",
                        entry.name,
                        entry.path.display()
                    ))
                );
            }
            DoctorFixOutcome::Unfixable(reason) => {
                println!("{}", red(&format!(" x {}: {}", entry.name, reason)));
                println!(
                    "{}",
                    yellow(&format!(
                        " Fix: repair the syntax manually in {}",
                        entry.path.display()
                    ))
                );
            }
        }
    }

    println!();
    println!(
        "{} reformatted, {} already clean, {} unfixable",
        report.reformatted(),
        report.unchanged(),
        report.unfixable()
    );
}
