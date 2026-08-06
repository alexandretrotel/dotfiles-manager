use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "dotfiles-manager",
    version = env!("CARGO_PKG_VERSION"),
    about = "A Rust-based command-line tool for dotfiles management with profiles."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Clone a dotfiles repo into ~/.dfm and restore it (onboard a new machine)")]
    Link(LinkArgs),

    #[command(about = "Backup system configurations and user data to a safe location")]
    Backup(BackupArgs),

    #[command(about = "Restore system state from a previously created backup")]
    Restore(RestoreArgs),

    #[command(about = "Switch to a different profile")]
    Use(UseArgs),

    #[command(about = "Manage profiles (list, create, delete)")]
    Profile(ProfileArgs),

    #[command(about = "Run git commands in the dfm repository")]
    Git(GitArgs),

    #[command(about = "Show the working tree status (shortcut for `dfm git status`)")]
    Status(PassthroughArgs),

    #[command(about = "Show changes (shortcut for `dfm git diff`)")]
    Diff(PassthroughArgs),

    #[command(about = "Stage, commit, and push to the dfm repository")]
    Sync(SyncArgs),

    #[command(about = "Validate dfm's registry files and check backups for drift")]
    Doctor(DoctorArgs),

    #[command(about = "Manage the encryption password in the system keychain")]
    Secret {
        #[command(subcommand)]
        action: SecretActions,
    },

    #[command(about = "Delete backup directories left behind by profiles that no longer exist")]
    Prune,

    #[command(about = "Open one of dfm's registry/config files in an editor")]
    Edit(EditArgs),
}

#[derive(Subcommand)]
pub enum SecretActions {
    #[command(about = "Store the encryption password in the system keychain")]
    Set,

    #[command(about = "Remove the encryption password from the system keychain")]
    Delete,
}

#[derive(Args)]
pub struct BackupArgs {
    #[arg(
        long,
        short = 'p',
        visible_short_alias = 'n',
        help = "Target a specific profile for backup"
    )]
    pub profile: Option<String>,
    #[arg(
        long,
        help = "Skip encrypted configs backup (will not prompt for password)"
    )]
    pub skip_encrypted: bool,
    #[arg(
        long,
        help = "Always prompt for the encryption password instead of using the one stored in the system keychain"
    )]
    pub ask_password: bool,
}

#[derive(Args)]
pub struct LinkArgs {
    #[arg(help = "GitHub repo to link: a URL or `owner/repo` shorthand")]
    pub repo: String,
    #[arg(
        long,
        help = "Skip encrypted configs restore (will not prompt for password)"
    )]
    pub skip_encrypted: bool,
    #[arg(
        long,
        help = "Always prompt for the encryption password instead of using the one stored in the system keychain"
    )]
    pub ask_password: bool,
}

#[derive(Args)]
pub struct RestoreArgs {
    #[arg(
        long,
        help = "Skip encrypted configs restore (will not prompt for password)"
    )]
    pub skip_encrypted: bool,
    #[arg(
        long,
        help = "Always prompt for the encryption password instead of using the one stored in the system keychain"
    )]
    pub ask_password: bool,
}

#[derive(Args)]
pub struct DoctorArgs {
    #[arg(
        long,
        help = "Skip encrypted configs validation (will not prompt for password)"
    )]
    pub skip_encrypted: bool,
    #[arg(
        long,
        help = "Always prompt for the encryption password instead of using the one stored in the system keychain"
    )]
    pub ask_password: bool,
    #[arg(
        long,
        help = "Rewrite dfm's own registry/config JSON files (config.registry.json, package.registry.json, encrypted.registry.json, profiles.json) as pretty-printed, sorted JSON. Never touches user-owned backed-up config files."
    )]
    pub fix: bool,
    #[arg(
        long,
        help = "Also check disabled registry entries in the backup consistency check"
    )]
    pub include_disabled: bool,
}

#[derive(Args)]
pub struct EditArgs {
    #[arg(help = "Which registry/config file to edit")]
    pub registry: RegistryChoice,
    #[arg(
        long,
        short = 'e',
        help = "Editor command to launch: a name like vi, nano, or emacs, or any custom binary/command (e.g. `code --wait`). Defaults to $VISUAL, then $EDITOR, then vi."
    )]
    pub editor: Option<String>,
}

/// Which of dfm's own registry/config files `dfm edit` opens.
#[derive(Clone, Copy, ValueEnum)]
pub enum RegistryChoice {
    #[value(help = "config.registry.json")]
    Config,
    #[value(help = "package.registry.json")]
    Package,
    #[value(help = "encrypted.registry.json")]
    Encrypted,
    #[value(help = "profiles.json")]
    Profiles,
}

impl From<RegistryChoice> for dotfiles_manager::edit::RegistryTarget {
    fn from(choice: RegistryChoice) -> Self {
        match choice {
            RegistryChoice::Config => dotfiles_manager::edit::RegistryTarget::Config,
            RegistryChoice::Package => dotfiles_manager::edit::RegistryTarget::Package,
            RegistryChoice::Encrypted => dotfiles_manager::edit::RegistryTarget::Encrypted,
            RegistryChoice::Profiles => dotfiles_manager::edit::RegistryTarget::Profiles,
        }
    }
}

#[derive(Args)]
pub struct GitArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
    pub args: Vec<String>,
}

/// Trailing args forwarded as-is to the underlying `git` invocation, for
/// `dfm status`/`dfm diff` shortcuts. Unlike [`GitArgs`], empty is valid
/// (e.g. plain `dfm status`).
#[derive(Args)]
pub struct PassthroughArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(Args)]
pub struct SyncArgs {
    #[arg(
        long,
        short = 'm',
        help = "Custom commit message; defaults to chore: sync dfm (<UTC date time>) when omitted"
    )]
    pub message: Option<String>,
}

#[derive(Args)]
pub struct UseArgs {
    #[arg(help = "Profile name to switch to")]
    pub profile: String,
}

#[derive(Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub action: Option<ProfileActions>,
}

#[derive(Subcommand)]
pub enum ProfileActions {
    #[command(about = "List all available profiles")]
    List,

    #[command(about = "Create a new profile")]
    Create {
        #[arg(help = "Name for the new profile")]
        name: String,
        #[arg(long, short = 'd', help = "Optional description for the profile")]
        description: Option<String>,
    },

    #[command(about = "Delete a profile")]
    Delete {
        #[arg(help = "Name of the profile to delete")]
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn doctor_with_flags_sets_them_true() {
        let cli = Cli::try_parse_from([
            "dfm",
            "doctor",
            "--fix",
            "--skip-encrypted",
            "--include-disabled",
        ])
        .unwrap();
        match cli.command {
            Some(Command::Doctor(args)) => {
                assert!(args.fix);
                assert!(args.skip_encrypted);
                assert!(!args.ask_password);
                assert!(args.include_disabled);
            }
            _ => panic!("expected Doctor command"),
        }
    }

    #[test]
    fn doctor_with_no_flags_defaults_false() {
        let cli = Cli::try_parse_from(["dfm", "doctor"]).unwrap();
        match cli.command {
            Some(Command::Doctor(args)) => {
                assert!(!args.fix);
                assert!(!args.skip_encrypted);
                assert!(!args.ask_password);
                assert!(!args.include_disabled);
            }
            _ => panic!("expected Doctor command"),
        }
    }

    #[test]
    fn link_requires_repo_argument() {
        let result = Cli::try_parse_from(["dfm", "link"]);
        assert!(result.is_err());
    }

    #[test]
    fn link_parses_repo_argument() {
        let cli = Cli::try_parse_from(["dfm", "link", "owner/repo"]).unwrap();
        match cli.command {
            Some(Command::Link(args)) => {
                assert_eq!(args.repo, "owner/repo");
                assert!(!args.skip_encrypted);
                assert!(!args.ask_password);
            }
            _ => panic!("expected Link command"),
        }
    }

    #[test]
    fn use_requires_profile_argument() {
        let result = Cli::try_parse_from(["dfm", "use"]);
        assert!(result.is_err());
    }

    #[test]
    fn use_parses_profile_argument() {
        let cli = Cli::try_parse_from(["dfm", "use", "work"]).unwrap();
        match cli.command {
            Some(Command::Use(args)) => assert_eq!(args.profile, "work"),
            _ => panic!("expected Use command"),
        }
    }

    #[test]
    fn git_requires_at_least_one_arg() {
        let result = Cli::try_parse_from(["dfm", "git"]);
        assert!(result.is_err());
    }

    #[test]
    fn git_passes_through_trailing_hyphen_values() {
        let cli = Cli::try_parse_from(["dfm", "git", "commit", "-m", "msg"]).unwrap();
        match cli.command {
            Some(Command::Git(args)) => {
                assert_eq!(args.args, vec!["commit", "-m", "msg"]);
            }
            _ => panic!("expected Git command"),
        }
    }

    #[test]
    fn status_allows_no_trailing_args() {
        let cli = Cli::try_parse_from(["dfm", "status"]).unwrap();
        match cli.command {
            Some(Command::Status(args)) => assert!(args.args.is_empty()),
            _ => panic!("expected Status command"),
        }
    }

    #[test]
    fn diff_passes_through_trailing_hyphen_values() {
        let cli = Cli::try_parse_from(["dfm", "diff", "--stat"]).unwrap();
        match cli.command {
            Some(Command::Diff(args)) => assert_eq!(args.args, vec!["--stat"]),
            _ => panic!("expected Diff command"),
        }
    }

    #[test]
    fn sync_message_defaults_to_none() {
        let cli = Cli::try_parse_from(["dfm", "sync"]).unwrap();
        match cli.command {
            Some(Command::Sync(args)) => assert_eq!(args.message, None),
            _ => panic!("expected Sync command"),
        }
    }

    #[test]
    fn sync_parses_short_message_flag() {
        let cli = Cli::try_parse_from(["dfm", "sync", "-m", "chore: update"]).unwrap();
        match cli.command {
            Some(Command::Sync(args)) => {
                assert_eq!(args.message, Some("chore: update".to_string()));
            }
            _ => panic!("expected Sync command"),
        }
    }

    #[test]
    fn backup_parses_profile_alias() {
        let cli = Cli::try_parse_from(["dfm", "backup", "-n", "personal"]).unwrap();
        match cli.command {
            Some(Command::Backup(args)) => {
                assert_eq!(args.profile, Some("personal".to_string()));
            }
            _ => panic!("expected Backup command"),
        }
    }

    #[test]
    fn profile_create_parses_name_and_description() {
        let cli = Cli::try_parse_from([
            "dfm",
            "profile",
            "create",
            "work",
            "--description",
            "work machine",
        ])
        .unwrap();
        match cli.command {
            Some(Command::Profile(args)) => match args.action {
                Some(ProfileActions::Create { name, description }) => {
                    assert_eq!(name, "work");
                    assert_eq!(description, Some("work machine".to_string()));
                }
                _ => panic!("expected Create action"),
            },
            _ => panic!("expected Profile command"),
        }
    }

    #[test]
    fn profile_delete_requires_name() {
        let result = Cli::try_parse_from(["dfm", "profile", "delete"]);
        assert!(result.is_err());
    }

    #[test]
    fn profile_with_no_action_is_none() {
        let cli = Cli::try_parse_from(["dfm", "profile"]).unwrap();
        match cli.command {
            Some(Command::Profile(args)) => assert!(args.action.is_none()),
            _ => panic!("expected Profile command"),
        }
    }

    #[test]
    fn secret_set_and_delete_parse() {
        let cli = Cli::try_parse_from(["dfm", "secret", "set"]).unwrap();
        match cli.command {
            Some(Command::Secret { action }) => assert!(matches!(action, SecretActions::Set)),
            _ => panic!("expected Secret command"),
        }

        let cli = Cli::try_parse_from(["dfm", "secret", "delete"]).unwrap();
        match cli.command {
            Some(Command::Secret { action }) => assert!(matches!(action, SecretActions::Delete)),
            _ => panic!("expected Secret command"),
        }
    }

    #[test]
    fn prune_parses_with_no_args() {
        let cli = Cli::try_parse_from(["dfm", "prune"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Prune)));
    }

    #[test]
    fn edit_requires_registry_argument() {
        let result = Cli::try_parse_from(["dfm", "edit"]);
        assert!(result.is_err());
    }

    #[test]
    fn edit_rejects_unknown_registry() {
        let result = Cli::try_parse_from(["dfm", "edit", "bogus"]);
        assert!(result.is_err());
    }

    #[test]
    fn edit_parses_registry_choice_and_defaults_editor_to_none() {
        let cli = Cli::try_parse_from(["dfm", "edit", "config"]).unwrap();
        match cli.command {
            Some(Command::Edit(args)) => {
                assert!(matches!(args.registry, RegistryChoice::Config));
                assert_eq!(args.editor, None);
            }
            _ => panic!("expected Edit command"),
        }
    }

    #[test]
    fn edit_parses_all_registry_choices() {
        for (arg, expected) in [
            ("config", "Config"),
            ("package", "Package"),
            ("encrypted", "Encrypted"),
            ("profiles", "Profiles"),
        ] {
            let cli = Cli::try_parse_from(["dfm", "edit", arg]).unwrap();
            match cli.command {
                Some(Command::Edit(args)) => {
                    let actual = match args.registry {
                        RegistryChoice::Config => "Config",
                        RegistryChoice::Package => "Package",
                        RegistryChoice::Encrypted => "Encrypted",
                        RegistryChoice::Profiles => "Profiles",
                    };
                    assert_eq!(actual, expected);
                }
                _ => panic!("expected Edit command"),
            }
        }
    }

    #[test]
    fn edit_parses_custom_editor_flag() {
        let cli =
            Cli::try_parse_from(["dfm", "edit", "profiles", "--editor", "code --wait"]).unwrap();
        match cli.command {
            Some(Command::Edit(args)) => {
                assert_eq!(args.editor, Some("code --wait".to_string()));
            }
            _ => panic!("expected Edit command"),
        }
    }

    #[test]
    fn edit_parses_short_editor_flag() {
        let cli = Cli::try_parse_from(["dfm", "edit", "config", "-e", "nano"]).unwrap();
        match cli.command {
            Some(Command::Edit(args)) => {
                assert_eq!(args.editor, Some("nano".to_string()));
            }
            _ => panic!("expected Edit command"),
        }
    }

    #[test]
    fn no_subcommand_is_none() {
        let cli = Cli::try_parse_from(["dfm"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn unknown_subcommand_is_err() {
        let result = Cli::try_parse_from(["dfm", "not-a-command"]);
        assert!(result.is_err());
    }
}
