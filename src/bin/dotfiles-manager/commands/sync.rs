use anstream::println;
use color_eyre::eyre::{Result, WrapErr};
use dotfiles_manager::Dfm;

use super::with_suggestions;
use crate::app::cli::SyncArgs;
use crate::app::output::{green, yellow};

/// Handle `dfm sync`.
pub fn run(ctx: &Dfm, args: SyncArgs) -> Result<()> {
    let report = dotfiles_manager::sync::run(ctx, args.message.as_deref())
        .map_err(with_suggestions)
        .wrap_err("Sync failed")?;

    if report.committed.is_none() {
        println!("{}", yellow("   No changes to commit"));
    }

    println!("{}", green("Sync complete"));
    Ok(())
}
