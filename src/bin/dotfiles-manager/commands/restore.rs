use anstream::println;
use color_eyre::eyre::{Result, WrapErr};
use dotfiles_manager::Dfm;
use dotfiles_manager::profiles::ActiveProfile;

use super::with_suggestions;
use crate::cli::RestoreArgs;
use crate::output::{green, print_section_with_summary};
use crate::prompt;

/// Handle `dfm restore`.
pub fn run(ctx: &Dfm, args: RestoreArgs) -> Result<()> {
    let profile = ActiveProfile::resolve(ctx, None);

    let password =
        prompt::optional_password(args.skip_encrypted, args.ask_password, "encrypted restore");

    println!("Restoring...");
    println!("   Target: {}", profile);

    let report = dotfiles_manager::restore::run(ctx, &profile, password.as_ref())
        .map_err(with_suggestions)
        .wrap_err("Restore failed")?;

    print_section_with_summary("Configurations", &report.configs);
    if let Some(encrypted) = &report.encrypted {
        print_section_with_summary("Encrypted configs", encrypted);
    }

    println!("{}", green("Restore complete"));
    Ok(())
}
