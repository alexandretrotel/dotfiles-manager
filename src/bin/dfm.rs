#[path = "dotfiles-manager/app/mod.rs"]
pub(crate) mod app;

use color_eyre::eyre::Result;

fn main() -> Result<()> {
    app::run()
}
