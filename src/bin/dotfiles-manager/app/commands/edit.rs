use anstream::println;
use color_eyre::eyre::Result;
use dotfiles_manager::Dfm;
use dotfiles_manager::edit::RegistryTarget;

use super::with_suggestions;
use crate::app::cli::EditArgs;

/// Handle `dfm edit`.
pub fn run(ctx: &Dfm, args: EditArgs) -> Result<()> {
    let target: RegistryTarget = args.registry.into();
    let path = target.path(ctx);

    println!("Opening {} ...", path.display());
    dotfiles_manager::edit::open(ctx, target, args.editor.as_deref()).map_err(with_suggestions)?;

    Ok(())
}
