#[path = "dotfiles-manager/app/mod.rs"]
pub(crate) mod app;

use color_eyre::eyre::Result;

/// Entry point for the `dfm` binary; delegates to the dotfiles-manager app.
fn main() -> Result<()> {
    app::run()
}
