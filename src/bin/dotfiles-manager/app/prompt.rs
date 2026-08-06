use std::io::{self, Write};

use age::secrecy::SecretString;
use anstream::eprintln;
use color_eyre::eyre::{Result, WrapErr, bail};

/// Read the encryption password from the terminal, optionally with a
/// confirmation prompt.
pub fn prompt_password(confirm: bool) -> Result<SecretString> {
    let password =
        rpassword::prompt_password("Enter encryption password: ").wrap_err("Read password")?;

    if password.is_empty() {
        bail!("Password cannot be empty");
    }

    if confirm {
        let confirmation = rpassword::prompt_password("Confirm encryption password: ")
            .wrap_err("Read password confirmation")?;
        if password != confirmation {
            bail!("Passwords do not match");
        }
    }

    Ok(SecretString::new(password.into()))
}

/// Resolve the encryption password: the stored keychain password unless
/// `ask_password` forces a prompt.
pub fn resolve_password(ask_password: bool, confirm_on_prompt: bool) -> Result<SecretString> {
    let stored = dotfiles_manager::encryption::keyring::get_stored_password();
    let had_stored = stored.is_some();
    if !ask_password && let Some(password) = stored {
        return Ok(password);
    }
    let password = prompt_password(confirm_on_prompt)?;
    if !had_stored {
        eprintln!(
            "Tip: run `dfm secret set` to save this password in your system keychain and skip prompts later."
        );
    }
    Ok(password)
}

/// Ask `question` as a `[y/N]` prompt. Any input other than exactly `y` or
/// `Y` (including a blank line) counts as "no" — the caller must opt in
/// explicitly, never by accident.
pub fn confirm(question: &str) -> Result<bool> {
    print!("{} [y/N]: ", question);
    io::stdout().flush().wrap_err("Flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .wrap_err("Read confirmation")?;

    Ok(matches!(input.trim(), "y" | "Y"))
}

/// Resolve the password for an optional encrypted step. Returns `None` when
/// the step is skipped, or when resolution fails — after printing a
/// "Skipping <step>" notice.
pub fn optional_password(skip: bool, ask_password: bool, step: &str) -> Option<SecretString> {
    if skip {
        return None;
    }
    match resolve_password(ask_password, false) {
        Ok(password) => Some(password),
        Err(e) => {
            eprintln!("Skipping {}: {}", step, e);
            None
        }
    }
}
