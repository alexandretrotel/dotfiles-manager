mod cli;
mod commands;
mod encryption;
mod errors;
mod profiles;
mod registry;
mod utils;

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands};
use commands::{backup, doctor, git, profile, restore, secret, sync, r#use};

pub fn run() -> color_eyre::eyre::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Backup(args)) => backup::run(args),
        Some(Commands::Restore(args)) => restore::run(args),
        Some(Commands::Use(args)) => r#use::run(args),
        Some(Commands::Profile(args)) => profile::run(args),
        Some(Commands::Git(args)) => git::run(args),
        Some(Commands::Sync(args)) => sync::run(args),
        Some(Commands::Doctor(args)) => doctor::run(args),
        Some(Commands::Secret { action }) => secret::run(action),
        None => {
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
    }
}
