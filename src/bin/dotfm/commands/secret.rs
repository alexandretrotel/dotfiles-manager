use color_eyre::eyre::{Result, WrapErr};

use super::with_suggestions;
use crate::cli::SecretActions;
use crate::output::green;
use crate::prompt;

pub fn run(action: SecretActions) -> Result<()> {
    match action {
        SecretActions::Set => {
            let password = prompt::prompt_password(true)
                .wrap_err("Read encryption password for system keychain")?;
            dotfm::encryption::keyring::set_stored_password(&password).map_err(with_suggestions)?;
            println!("{}", green("Secret set complete"));
        }
        SecretActions::Delete => {
            dotfm::encryption::keyring::clear_stored_password().map_err(with_suggestions)?;
            println!("{}", green("Secret delete complete"));
        }
    }
    Ok(())
}
