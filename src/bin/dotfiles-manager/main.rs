pub(crate) mod app;

use color_eyre::eyre::Result;

/// Entry point for the `dotfiles-manager` binary; delegates to the app runner.
fn main() -> Result<()> {
    app::run()
}
