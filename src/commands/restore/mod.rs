use crate::commands::core::Command;
use crate::encryption::resolve_encryption_password;
use crate::profiles::ActiveProfile;
use crate::registry::config::ConfigRegistry;
use crate::utils::{
    display::{green, yellow},
    paths::get_config_registry_path,
};
mod config;
mod encrypted;

struct RestoreTask {
    profile: ActiveProfile,
    skip_encrypted: bool,
    ask_password: bool,
}

impl RestoreTask {
    fn new(profile: ActiveProfile, skip_encrypted: bool, ask_password: bool) -> Self {
        Self {
            profile,
            skip_encrypted,
            ask_password,
        }
    }
}

impl Command for RestoreTask {
    fn name(&self) -> &str {
        "Restore"
    }

    fn execute(&mut self) -> eyre::Result<()> {
        let config_registry_path = get_config_registry_path();
        let config_registry = ConfigRegistry::load_or_create(&config_registry_path)?;

        let enabled_entries: Vec<_> = config_registry.get_enabled_entries().collect();
        println!(
            "   Configurations: {} entries ({})",
            enabled_entries.len(),
            self.profile
        );

        let mut restored_count = 0;
        let mut skipped_count = 0;

        for (id, entry) in enabled_entries {
            let target_path = &entry.target_path;
            match self.profile.resolve_source(&entry.source_path) {
                Some(resolved) => {
                    if config::restore_configs(&resolved.path, target_path) {
                        restored_count += 1;
                        println!("     {} {}", green("✔"), entry.source_path);
                    } else {
                        skipped_count += 1;
                    }
                }
                None => {
                    println!(
                        "{}",
                        yellow(&format!(
                            "     skipped {} ({}): no backup in any layer",
                            entry.source_path, id
                        ))
                    );
                    skipped_count += 1;
                }
            }
        }

        if !self.skip_encrypted {
            match resolve_encryption_password(self.ask_password, false) {
                Ok(password) => {
                    let (encrypted_restored, encrypted_skipped) =
                        encrypted::restore_encrypted_configs(&self.profile, &password);
                    restored_count += encrypted_restored;
                    skipped_count += encrypted_skipped;
                }
                Err(e) => {
                    eprintln!("Skipping encrypted restore: {}", e);
                }
            }
        }

        println!(
            "Restore complete. {} restored, {} skipped",
            restored_count, skipped_count
        );

        Ok(())
    }
}

pub(crate) fn run(args: crate::cli::RestoreArgs) {
    use crate::commands::core::CommandExecutor;
    let profile = args.resolve_profile();
    CommandExecutor::run(&mut RestoreTask::new(
        profile,
        args.skip_encrypted,
        args.ask_password,
    ));
}
