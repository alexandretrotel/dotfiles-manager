use color_eyre::eyre::Result;
use dotfiles_manager::Dfm;

use super::with_suggestions;
use crate::app::cli::GitArgs;

/// Handle `dfm git <args>`.
pub fn run(ctx: &Dfm, args: GitArgs) -> Result<()> {
    dotfiles_manager::git::passthrough(ctx, &args.args).map_err(with_suggestions)?;
    Ok(())
}
