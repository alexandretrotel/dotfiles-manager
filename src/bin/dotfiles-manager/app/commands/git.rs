use color_eyre::eyre::Result;
use dotfiles_manager::Dfm;

use super::with_suggestions;
use crate::app::cli::{GitArgs, PassthroughArgs};

/// Handle `dfm git <args>`.
pub fn run(ctx: &Dfm, args: GitArgs) -> Result<()> {
    dotfiles_manager::git::passthrough(ctx, &args.args).map_err(with_suggestions)?;
    Ok(())
}

/// Handle `dfm status` (shortcut for `dfm git status`).
pub fn status(ctx: &Dfm, args: PassthroughArgs) -> Result<()> {
    run_with_leading_subcommand(ctx, "status", args)
}

/// Handle `dfm diff` (shortcut for `dfm git diff`).
pub fn diff(ctx: &Dfm, args: PassthroughArgs) -> Result<()> {
    run_with_leading_subcommand(ctx, "diff", args)
}

/// Run `git <subcommand> <args.args...>` in the dfm repository.
fn run_with_leading_subcommand(ctx: &Dfm, subcommand: &str, args: PassthroughArgs) -> Result<()> {
    let mut full_args = vec![subcommand.to_string()];
    full_args.extend(args.args);
    dotfiles_manager::git::passthrough(ctx, &full_args).map_err(with_suggestions)?;
    Ok(())
}
