use crate::cli::SecretActions;
use crate::commands::core::{Command, CommandExecutor};
use crate::encryption::{clear_stored_encryption_password, persist_encryption_password};
use color_eyre::eyre::Result;

struct SecretSetTask;

impl Command for SecretSetTask {
    fn name(&self) -> &str {
        "Secret set"
    }

    fn execute(&mut self) -> Result<()> {
        persist_encryption_password()
    }
}

struct SecretDeleteTask;

impl Command for SecretDeleteTask {
    fn name(&self) -> &str {
        "Secret delete"
    }

    fn execute(&mut self) -> Result<()> {
        clear_stored_encryption_password()
    }
}

pub(crate) fn run(action: SecretActions) -> color_eyre::eyre::Result<()> {
    match action {
        SecretActions::Set => CommandExecutor::run(&mut SecretSetTask),
        SecretActions::Delete => CommandExecutor::run(&mut SecretDeleteTask),
    }
}
