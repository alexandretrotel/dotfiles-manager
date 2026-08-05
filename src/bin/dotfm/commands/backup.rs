use age::secrecy::SecretString;
use color_eyre::eyre::{Result, WrapErr};
use dotfm::Dotfm;
use dotfm::profiles::ActiveProfile;

use super::with_suggestions;
use crate::cli::BackupArgs;
use crate::output::{green, print_section_with_summary};
use crate::prompt;

pub fn run(ctx: &Dotfm, args: BackupArgs) -> Result<()> {
    let profile = ActiveProfile::resolve(ctx, args.profile.as_deref());
    let password = resolve_backup_password(args.skip_encrypted, args.ask_password)?;

    println!("Backing up...");
    println!("   Target: {}", profile);

    let report = dotfm::backup::run(ctx, &profile, password.as_ref())
        .map_err(with_suggestions)
        .wrap_err("Backup failed")?;

    if report.repo.initialized {
        println!("Initialized git repository in {}", ctx.root().display());
    }

    print_section_with_summary("Configurations", &report.configs);
    print_section_with_summary("Package managers", &report.packages);
    if let Some(encrypted) = &report.encrypted {
        print_section_with_summary("Encrypted configs", encrypted);
    }

    println!("{}", green("Backup complete"));
    Ok(())
}

fn resolve_backup_password(
    skip_encrypted: bool,
    ask_password: bool,
) -> Result<Option<SecretString>> {
    if skip_encrypted {
        return Ok(None);
    }
    prompt::resolve_password(ask_password, true)
        .wrap_err("Prompt for encryption password")
        .map(Some)
}
