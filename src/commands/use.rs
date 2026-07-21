use crate::cli::UseArgs;
use crate::commands::core::{Command, CommandExecutor};
use crate::profiles::{
    ProfileConfig, clear_active_profile, get_active_profile_name, set_active_profile,
};
use color_eyre::Help;
use color_eyre::eyre::eyre;

const COMMON: &str = "common";
const NONE: &str = "none";

struct UseTask {
    profile_name: String,
}

impl UseTask {
    fn new(profile_name: String) -> Self {
        Self { profile_name }
    }

    fn is_clearing_profile(&self) -> bool {
        self.profile_name == COMMON || self.profile_name == NONE
    }
}

impl Command for UseTask {
    fn name(&self) -> &str {
        "Use"
    }

    fn execute(&mut self) -> color_eyre::eyre::Result<()> {
        let config = ProfileConfig::load_or_default();

        if self.is_clearing_profile() {
            clear_active_profile()?;
            println!("Switched to {} (no active profile)", COMMON);
            return Ok(());
        }

        if !config.profile_exists(&self.profile_name) {
            return Err(eyre!("Profile '{}' does not exist", self.profile_name))
                .suggestion(format!(
                    "Create it with: mntn profile create {}",
                    self.profile_name
                ))
                .suggestion("List available profiles with: mntn profile list");
        }

        let current = get_active_profile_name();
        if current.as_deref() == Some(&self.profile_name) {
            println!("Already using profile '{}'", self.profile_name);
            return Ok(());
        }

        set_active_profile(&self.profile_name)?;

        println!("Switched to profile '{}'", self.profile_name);
        println!();
        println!("Run 'mntn restore' to apply this profile's configurations");

        Ok(())
    }
}

pub(crate) fn run(args: UseArgs) -> color_eyre::eyre::Result<()> {
    let mut task = UseTask::new(args.profile);
    CommandExecutor::run(&mut task)
}
