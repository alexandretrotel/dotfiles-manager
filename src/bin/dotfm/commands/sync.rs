use color_eyre::eyre::{Result, WrapErr};
use dotfm::Dotfm;

use super::with_suggestions;
use crate::cli::SyncArgs;
use crate::output::{green, yellow};

pub fn run(ctx: &Dotfm, args: SyncArgs) -> Result<()> {
    let report = dotfm::sync::run(ctx, args.message.as_deref())
        .map_err(with_suggestions)
        .wrap_err("Sync failed")?;

    if report.committed.is_none() {
        println!("{}", yellow("   No changes to commit"));
    }

    println!("{}", green("Sync complete"));
    Ok(())
}
