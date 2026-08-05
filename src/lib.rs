//! dotfm — dotfiles management with profiles.
//!
//! This crate is both a library and the `dotfm` CLI (behind the `cli`
//! feature, enabled by default). The library performs no terminal I/O:
//! operations take a [`Dotfm`] context plus plain arguments and return
//! report structs describing what happened; rendering and prompting live in
//! the binary.
//!
//! ```no_run
//! use dotfm::{Dotfm, profiles::ActiveProfile};
//!
//! let ctx = Dotfm::new()?;
//! let profile = ActiveProfile::resolve(&ctx, None);
//! let report = dotfm::backup::run(&ctx, &profile, None)?;
//! println!("{} configs backed up", report.configs.succeeded());
//! # Ok::<(), dotfm::Error>(())
//! ```

/// Back up configs, package lists, and encrypted configs to the dotfm root.
pub mod backup;
/// The [`Dotfm`] root-directory handle and its well-known subpaths.
mod context;
/// Validate the dotfm root (registries, JSON syntax, layer resolution,
/// backup/current drift) and repair what can be repaired.
pub mod doctor;
/// Age-passphrase file encryption and the system-keychain-backed password
/// store.
pub mod encryption;
/// The crate-wide [`Error`] type and [`Result`] alias.
mod error;
/// Thin wrapper around the `git` CLI used to manage the dotfm root repo.
pub mod git;
/// Named profiles layered on top of the `common` backup, and resolving
/// which layer a given source file comes from.
pub mod profiles;
/// JSON-backed registries of what to back up: configs, package-manager
/// exports, and encrypted files.
pub mod registry;
/// Shared result types returned by backup/restore runs.
mod report;
/// Restore configs and encrypted configs from the dotfm root.
pub mod restore;
/// Stage, commit, and push the dotfm root's git repository.
pub mod sync;
/// Small filesystem/process/text helpers shared across modules.
mod utils;

pub use context::{Dotfm, ENCRYPTED_BUNDLE_FILE, PROFILE_CONFIG_FILE};
pub use error::{Error, Result};
pub use report::{ItemOutcome, ItemStatus, SectionReport};
