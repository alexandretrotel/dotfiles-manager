use anstream::println;
use color_eyre::eyre::{Result, WrapErr};
use dotfiles_manager::Dfm;
use dotfiles_manager::profiles;

use crate::app::output::green;
use crate::app::prompt;

/// Handle `dfm prune`.
pub fn run(ctx: &Dfm) -> Result<()> {
    let orphans = profiles::find_orphaned_profiles(ctx).wrap_err("Scan profile directories")?;

    if orphans.is_empty() {
        println!("No orphaned profile directories found");
        return Ok(());
    }

    let noun = if orphans.len() == 1 {
        "directory"
    } else {
        "directories"
    };

    println!("Found {} orphaned profile {}:", orphans.len(), noun);
    for orphan in &orphans {
        println!("   {} ({})", orphan.name, orphan.directory.display());
    }
    println!();
    println!(
        "These backup directories have no matching profile in profiles.json and will be permanently deleted."
    );

    let confirmed = prompt::confirm(&format!("Delete {} orphaned profile {}?", orphans.len(), noun))?;

    if !confirmed {
        println!("Aborted, nothing was deleted");
        return Ok(());
    }

    let pruned = profiles::prune_orphaned_profiles(ctx).wrap_err("Delete orphaned profiles")?;
    for profile in &pruned {
        println!("   Deleted {} ({})", profile.name, profile.directory.display());
    }
    println!("{}", green("Prune complete"));
    Ok(())
}
