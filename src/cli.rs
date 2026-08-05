use clap::{Args, Parser, Subcommand};

use crate::profiles::ActiveProfile;

#[derive(Parser)]
#[command(
    name = "dotfm",
    version = env!("CARGO_PKG_VERSION"),
    about = "A Rust-based command-line tool for dotfiles management with profiles."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    #[command(about = "Backup system configurations and user data to a safe location")]
    Backup(BackupArgs),

    #[command(about = "Restore system state from a previously created backup")]
    Restore(RestoreArgs),

    #[command(about = "Switch to a different profile")]
    Use(UseArgs),

    #[command(about = "Manage profiles (list, create, delete)")]
    Profile(ProfileArgs),

    #[command(about = "Run git commands in the dotfm repository")]
    Git(GitArgs),

    #[command(about = "Stage, commit, and push to the dotfm repository")]
    Sync(SyncArgs),

    #[command(
        about = "Diagnose JSON configs, symlinks, and registry files (run `doctor fix` to repair)"
    )]
    Doctor(DoctorArgs),

    #[command(about = "Manage the encryption password in the system keychain")]
    Secret {
        #[command(subcommand)]
        action: SecretActions,
    },
}

#[derive(Subcommand)]
pub(crate) enum SecretActions {
    #[command(about = "Store the encryption password in the system keychain")]
    Set,

    #[command(about = "Remove the encryption password from the system keychain")]
    Delete,
}

#[derive(Args)]
pub(crate) struct BackupArgs {
    #[arg(long, short = 'p', help = "Target a specific profile for backup")]
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

impl BackupArgs {
    pub fn resolve_profile(&self) -> ActiveProfile {
        ActiveProfile::resolve(self.profile.as_deref())
    }
}

#[derive(Args)]
pub(crate) struct RestoreArgs {
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

impl RestoreArgs {
    pub fn resolve_profile(&self) -> ActiveProfile {
        ActiveProfile::resolve(None)
    }
}

#[derive(Args)]
pub(crate) struct DoctorArgs {
    #[command(subcommand)]
    pub action: Option<DoctorActions>,
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
}

impl DoctorArgs {
    pub fn resolve_profile(&self) -> ActiveProfile {
        ActiveProfile::resolve(None)
    }
}

#[derive(Subcommand)]
pub(crate) enum DoctorActions {
    #[command(
        about = "Reformat valid JSON config files with serde_json's pretty printer (cannot repair true syntax errors)"
    )]
    Fix(DoctorFixArgs),
}

#[derive(Args)]
pub(crate) struct DoctorFixArgs {
    #[arg(long, help = "Show which files would change without writing anything")]
    pub dry_run: bool,
}

impl DoctorFixArgs {
    pub fn resolve_profile(&self) -> ActiveProfile {
        ActiveProfile::resolve(None)
    }
}

#[derive(Args)]
pub(crate) struct GitArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
    pub args: Vec<String>,
}

#[derive(Args)]
pub(crate) struct SyncArgs {
    #[arg(
        long,
        short = 'm',
        help = "Custom commit message; defaults to chore: sync dotfm (<UTC date time>) when omitted"
    )]
    pub message: Option<String>,
}

#[derive(Args)]
pub(crate) struct UseArgs {
    #[arg(help = "Profile name to switch to")]
    pub profile: String,
}

#[derive(Args)]
pub(crate) struct ProfileArgs {
    #[command(subcommand)]
    pub action: Option<ProfileActions>,
}

#[derive(Subcommand)]
pub(crate) enum ProfileActions {
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
