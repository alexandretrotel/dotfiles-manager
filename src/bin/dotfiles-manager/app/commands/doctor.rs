use anstream::{eprintln, println};
use color_eyre::eyre::{Result, eyre};
use dotfiles_manager::Dfm;
use dotfiles_manager::doctor::FixedFile;
use dotfiles_manager::profiles::{ActiveProfile, ProfileConfig};

use crate::app::cli::DoctorArgs;
use crate::app::output::{green, print_doctor_report, print_fix_report, red};
use crate::app::prompt;

/// Handle `dfm doctor`.
pub fn run(ctx: &Dfm, args: DoctorArgs) -> Result<()> {
    if let Ok(true) = ProfileConfig::save_default_if_missing(ctx) {
        println!(
            "Created default profile config at {}",
            ctx.profiles_config_path().display()
        );
    }

    if args.fix {
        let fixed = dotfiles_manager::doctor::fix(ctx);
        print_fix_report(&fixed);
        println!();
        if fixed.iter().any(FixedFile::failed) {
            return Err(eyre!("Fix failed for one or more files"));
        }
    }

    let profile = ActiveProfile::resolve(ctx, None);

    validate(
        ctx,
        &profile,
        args.skip_encrypted,
        args.ask_password,
        args.include_disabled,
    )
}

/// Handle `dfm doctor` (validation).
fn validate(
    ctx: &Dfm,
    profile: &ActiveProfile,
    skip_encrypted: bool,
    ask_password: bool,
    include_disabled: bool,
) -> Result<()> {
    println!("Validating configuration...");
    println!("   Profile: {}", profile);

    let password =
        prompt::optional_password(skip_encrypted, ask_password, "encrypted file validation");

    let report =
        dotfiles_manager::doctor::validate(ctx, profile, password.as_ref(), include_disabled);
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
