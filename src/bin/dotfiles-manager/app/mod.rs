pub(crate) mod cli;
pub(crate) mod commands;
pub(crate) mod output;
pub(crate) mod prompt;

use anstream::println;
use clap::{CommandFactory, Parser};
use color_eyre::eyre::Result;
use dotfiles_manager::Dfm;

use self::cli::{Cli, Command};

/// Parse CLI args and dispatch to the matching command handler.
pub fn run() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let ctx = Dfm::new()?;

    match cli.command {
        Some(Command::Link(args)) => commands::link::run(&ctx, args),
        Some(Command::Backup(args)) => commands::backup::run(&ctx, args),
        Some(Command::Restore(args)) => commands::restore::run(&ctx, args),
        Some(Command::Use(args)) => commands::r#use::run(&ctx, args),
        Some(Command::Profile(args)) => commands::profile::run(&ctx, args),
        Some(Command::Git(args)) => commands::git::run(&ctx, args),
        Some(Command::Status(args)) => commands::git::status(&ctx, args),
        Some(Command::Diff(args)) => commands::git::diff(&ctx, args),
        Some(Command::Sync(args)) => commands::sync::run(&ctx, args),
        Some(Command::Doctor(args)) => commands::doctor::run(&ctx, args),
        Some(Command::Secret { action }) => commands::secret::run(action),
        Some(Command::Prune) => commands::prune::run(&ctx),
        Some(Command::Edit(args)) => commands::edit::run(&ctx, args),
        None => {
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
    }
}
