use anstream::println;
use color_eyre::eyre::{Result, WrapErr};
use dotfiles_manager::Dfm;
use dotfiles_manager::profiles::ActiveProfile;

use super::with_suggestions;
use crate::cli::LinkArgs;
use crate::output::{green, print_section_with_summary};
use crate::prompt;

/// Handle `dfm link <repo>`.
pub fn run(ctx: &Dfm, args: LinkArgs) -> Result<()> {
    println!("Linking...");

    let link_report = dotfiles_manager::link::run(ctx, &args.repo)
        .map_err(with_suggestions)
        .wrap_err("Link failed")?;
    println!(
        "   Cloned {} into {}",
        link_report.url,
        ctx.root().display()
    );

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

    println!("{}", green("Link complete"));
    Ok(())
}
