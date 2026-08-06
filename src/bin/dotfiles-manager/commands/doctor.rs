use anstream::{eprintln, println};
use color_eyre::eyre::{Result, WrapErr, eyre};
use dotfiles_manager::Dfm;
use dotfiles_manager::profiles::{ActiveProfile, ProfileConfig};

use super::with_suggestions;
use crate::cli::{DoctorActions, DoctorArgs};
use crate::output::{green, print_doctor_fix_report, print_doctor_report, red};
use crate::prompt;

/// Handle `dfm doctor` (and `doctor fix`).
pub fn run(ctx: &Dfm, args: DoctorArgs) -> Result<()> {
    if let Ok(true) = ProfileConfig::save_default_if_missing(ctx) {
        println!(
            "Created default profile config at {}",
            ctx.profiles_config_path().display()
        );
    }

    let profile = ActiveProfile::resolve(ctx, None);

    match args.action {
        Some(DoctorActions::Fix) => fix(ctx, &profile),
        None => validate(ctx, &profile, args.skip_encrypted, args.ask_password),
    }
}

/// Handle `dfm doctor fix`.
fn fix(ctx: &Dfm, profile: &ActiveProfile) -> Result<()> {
    println!("Reformatting JSON configs...");
    println!("   Profile: {}", profile);

    let report = dotfiles_manager::doctor::fix_json_configs(ctx, profile)
        .map_err(with_suggestions)
        .wrap_err("Doctor fix failed")?;

    if report.entries.is_empty() {
        println!("{}", green("No JSON config files to format"));
        return Ok(());
    }

    print_doctor_fix_report(&report);

    if report.unfixable() > 0 {
        return Err(eyre!(
            "{} file(s) have syntax errors, this cannot be repaired",
            report.unfixable()
        ));
    }

    println!("{}", green("Doctor fix complete"));
    Ok(())
}

/// Handle `dfm doctor` (validation, the default action).
fn validate(
    ctx: &Dfm,
    profile: &ActiveProfile,
    skip_encrypted: bool,
    ask_password: bool,
) -> Result<()> {
    println!("Validating configuration...");
    println!("   Profile: {}", profile);

    let password =
        prompt::optional_password(skip_encrypted, ask_password, "encrypted file validation");

    let report = dotfiles_manager::doctor::validate(ctx, profile, password.as_ref());
    println!();
    print_doctor_report(&report);
    println!();

    let error_count = report.error_count();
    let warning_count = report.warning_count();

    if error_count > 0 {
        return Err(eyre!(
            "Validation failed: {} error(s), {} warning(s)",
            error_count,
            warning_count
        ));
    }

    if warning_count > 0 {
        eprintln!(
            "{}",
            red(&format!(
                "Validation complete: {} warning(s)",
                warning_count
            ))
        );
    } else {
        println!("{}", green("All checks passed"));
    }

    Ok(())
}
