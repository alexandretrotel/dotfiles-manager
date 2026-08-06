use clap::{Args, Parser, Subcommand};

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

    #[command(about = "Stage, commit, and push to the dfm repository")]
    Sync(SyncArgs),

    #[command(about = "Diagnose symlinks and registry files")]
    Doctor(DoctorArgs),

    #[command(about = "Manage the encryption password in the system keychain")]
    Secret {
        #[command(subcommand)]
        action: SecretActions,
    },
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
}

#[derive(Args)]
pub struct GitArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
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
