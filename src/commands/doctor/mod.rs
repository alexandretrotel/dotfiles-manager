use crate::cli::DoctorActions;
use crate::commands::core::{Command, CommandExecutor};
use crate::profiles::{ActiveProfile, ProfileConfig};
use crate::utils::display::{green, red};

mod fix;
mod types;
mod utils;
mod validators;

use validators::ValidationSuite;

struct DoctorTask {
    profile: ActiveProfile,
    skip_encrypted: bool,
    ask_password: bool,
}

impl DoctorTask {
    fn new(profile: ActiveProfile, skip_encrypted: bool, ask_password: bool) -> Self {
        Self {
            profile,
            skip_encrypted,
            ask_password,
        }
    }
}

impl Command for DoctorTask {
    fn name(&self) -> &str {
        "Doctor"
    }

    fn execute(&mut self) -> eyre::Result<()> {
        println!("Validating configuration...");
        println!("   Profile: {}", self.profile);
        println!("Starting validation");

        let validator =
            ValidationSuite::new(self.profile.clone(), self.skip_encrypted, self.ask_password);
        let report = validator.run_all();
        println!();
        report.print();
        println!();
        let error_count = report.error_count();
        let warning_count = report.warning_count();
        if error_count == 0 && warning_count == 0 {
            println!("{}", green("All checks passed"));
        } else if error_count == 0 {
            eprintln!(
                "{}",
                red(&format!(
                    "Validation complete: {} error(s), {} warning(s)",
                    error_count, warning_count
                ))
            );
        }
        if error_count > 0 {
            return Err(eyre::eyre!(
                "Validation failed: {} error(s), {} warning(s)",
                error_count,
                warning_count
            ));
        }

        Ok(())
    }
}

pub(crate) fn run(args: crate::cli::DoctorArgs) {
    if let Ok(true) = ProfileConfig::save_default_if_missing() {
        println!("Created default profile config at ~/.mntn/profiles.json");
    }

    match args.action {
        Some(DoctorActions::Fix(fix_args)) => {
            let profile = fix_args.resolve_profile();
            CommandExecutor::run(&mut fix::FixTask::new(profile, fix_args.dry_run));
        }
        None => {
            let profile = args.resolve_profile();
            CommandExecutor::run(&mut DoctorTask::new(
                profile,
                args.skip_encrypted,
                args.ask_password,
            ));
        }
    }
}
