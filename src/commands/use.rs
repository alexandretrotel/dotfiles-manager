use crate::cli::UseArgs;
use crate::commands::core::{Command, CommandExecutor};
use crate::profiles::{
    ProfileConfig, clear_active_profile, get_active_profile_name, set_active_profile,
};
use eyre::bail;

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

    fn execute(&mut self) -> eyre::Result<()> {
        let config = ProfileConfig::load_or_default();

        if self.is_clearing_profile() {
            clear_active_profile()?;
            println!("Switched to {} (no active profile)", COMMON);
            return Ok(());
        }

        if !config.profile_exists(&self.profile_name) {
            println!();
            println!("Create it with: mntn profile create {}", self.profile_name);
            println!("   Or list available profiles: mntn profile list");
            bail!("Profile '{}' does not exist", self.profile_name);
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

pub(crate) fn run(args: UseArgs) {
    let mut task = UseTask::new(args.profile);
    CommandExecutor::run(&mut task);
}
