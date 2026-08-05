use color_eyre::eyre::Result;
use dotfm::Dotfm;

use super::with_suggestions;
use crate::cli::GitArgs;

/// Handle `dotfm git <args>`.
pub fn run(ctx: &Dotfm, args: GitArgs) -> Result<()> {
    dotfm::git::passthrough(ctx, &args.args).map_err(with_suggestions)?;
    Ok(())
}
