use anstream::println;
use color_eyre::eyre::{Result, WrapErr};

use super::with_suggestions;
use crate::app::cli::SecretActions;
use crate::app::output::green;
use crate::app::prompt;

/// Handle `dfm secret` (set/delete the stored encryption password).
pub fn run(action: SecretActions) -> Result<()> {
    match action {
        SecretActions::Set => {
            let password = prompt::prompt_password(true)
                .wrap_err("Read encryption password for system keychain")?;
            dotfiles_manager::encryption::keyring::set_stored_password(&password)
                .map_err(with_suggestions)?;
            println!("{}", green("Secret set complete"));
        }
        SecretActions::Delete => {
            dotfiles_manager::encryption::keyring::clear_stored_password()
                .map_err(with_suggestions)?;
            println!("{}", green("Secret delete complete"));
        }
    }
    Ok(())
}
