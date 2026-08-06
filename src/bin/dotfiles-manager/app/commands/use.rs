use anstream::println;
use color_eyre::eyre::Result;
use dotfiles_manager::Dfm;
use dotfiles_manager::profiles::{SwitchProfileOutcome, switch_profile};

use super::with_suggestions;
use crate::app::cli::UseArgs;

/// Handle `dfm use <profile>`.
pub fn run(ctx: &Dfm, args: UseArgs) -> Result<()> {
    match switch_profile(ctx, &args.profile).map_err(with_suggestions)? {
        SwitchProfileOutcome::Cleared => {
            println!("Switched to common (no active profile)");
        }
        SwitchProfileOutcome::AlreadyActive(name) => {
            println!("Already using profile '{}'", name);
        }
        SwitchProfileOutcome::Switched(name) => {
            println!("Switched to profile '{}'", name);
            println!();
            println!("Run 'dfm restore' to apply this profile's configurations");
        }
    }
    Ok(())
}
