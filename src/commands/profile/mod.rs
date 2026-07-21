use crate::cli::{ProfileActions, ProfileArgs};
use crate::commands::core::{Command, CommandExecutor};

mod create;
mod delete;
mod list;

struct ProfileListTask;

impl Command for ProfileListTask {
    fn name(&self) -> &str {
        "Profile list"
    }

    fn execute(&mut self) -> color_eyre::eyre::Result<()> {
        list::list_profiles()
    }
}

struct ProfileCreateTask {
    name: String,
    description: Option<String>,
}

impl ProfileCreateTask {
    fn new(name: String, description: Option<String>) -> Self {
        Self { name, description }
    }
}

impl Command for ProfileCreateTask {
    fn name(&self) -> &str {
        "Profile create"
    }

    fn execute(&mut self) -> color_eyre::eyre::Result<()> {
        create::create_profile(&self.name, self.description.clone())
    }
}

struct ProfileDeleteTask {
    name: String,
}

impl ProfileDeleteTask {
    fn new(name: String) -> Self {
        Self { name }
    }
}

impl Command for ProfileDeleteTask {
    fn name(&self) -> &str {
        "Profile delete"
    }

    fn execute(&mut self) -> color_eyre::eyre::Result<()> {
        delete::delete_profile(&self.name)
    }
}

struct ProfileShowTask;

impl Command for ProfileShowTask {
    fn name(&self) -> &str {
        "Profile"
    }

    fn execute(&mut self) -> color_eyre::eyre::Result<()> {
        let current = crate::profiles::get_active_profile_name();
        match current {
            Some(name) => println!("Active profile: {}", name),
            None => println!("No active profile (using common only)"),
        }
        println!();
        list::list_profiles()?;
        println!();
        println!("Use 'mntn use <profile>' to switch profiles");
        Ok(())
    }
}

pub(crate) fn run(args: ProfileArgs) -> color_eyre::eyre::Result<()> {
    match args.action {
        Some(ProfileActions::List) => CommandExecutor::run(&mut ProfileListTask),
        Some(ProfileActions::Create { name, description }) => {
            CommandExecutor::run(&mut ProfileCreateTask::new(name, description))
        }
        Some(ProfileActions::Delete { name }) => {
            CommandExecutor::run(&mut ProfileDeleteTask::new(name))
        }
        None => CommandExecutor::run(&mut ProfileShowTask),
    }
}
