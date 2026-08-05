mod cli;
mod commands;
mod output;
mod prompt;

use clap::{CommandFactory, Parser};
use color_eyre::eyre::Result;
use dotfm::Dotfm;

use crate::cli::{Cli, Command};

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let ctx = Dotfm::new()?;

    match cli.command {
        Some(Command::Backup(args)) => commands::backup::run(&ctx, args),
        Some(Command::Restore(args)) => commands::restore::run(&ctx, args),
        Some(Command::Use(args)) => commands::r#use::run(&ctx, args),
        Some(Command::Profile(args)) => commands::profile::run(&ctx, args),
        Some(Command::Git(args)) => commands::git::run(&ctx, args),
        Some(Command::Sync(args)) => commands::sync::run(&ctx, args),
        Some(Command::Doctor(args)) => commands::doctor::run(&ctx, args),
        Some(Command::Secret { action }) => commands::secret::run(action),
        None => {
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
    }
}
