pub mod backup;
pub mod doctor;
pub mod git;
pub mod profile;
pub mod restore;
pub mod secret;
pub mod sync;
pub mod r#use;

use color_eyre::Help;
use color_eyre::eyre::eyre;
use dotfm::Error;

/// Convert a library error into an eyre report, attaching CLI suggestions
/// for the errors that have an obvious next step.
pub(crate) fn with_suggestions(e: Error) -> color_eyre::eyre::Report {
    match &e {
        Error::ProfileNotFound(name) => {
            let create = format!("Create it with: dotfm profile create {}", name);
            eyre!(e)
                .suggestion(create)
                .suggestion("List available profiles with: dotfm profile list")
        }
        Error::NoGitRepository { .. } => {
            eyre!(e).suggestion("Run 'dotfm backup' to initialize it.")
        }
        _ => eyre!(e),
    }
}
