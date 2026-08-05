use color_eyre::eyre::Result;
use dotfm::Dotfm;

use super::with_suggestions;
use crate::cli::GitArgs;

pub fn run(ctx: &Dotfm, args: GitArgs) -> Result<()> {
    dotfm::git::passthrough(ctx, &args.args).map_err(with_suggestions)?;
    Ok(())
}
