use color_eyre::eyre::Result;
use dotfm::Dotfm;
use dotfm::profiles::{SwitchProfileOutcome, switch_profile};

use super::with_suggestions;
use crate::cli::UseArgs;

/// Handle `dotfm use <profile>`.
pub fn run(ctx: &Dotfm, args: UseArgs) -> Result<()> {
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
            println!("Run 'dotfm restore' to apply this profile's configurations");
        }
    }
    Ok(())
}
