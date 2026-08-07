//! dotfiles-manager — dotfiles management with profiles.
//!
//! This crate is both a library and the `dotfiles-manager` CLI (aliased as
//! `dfm`, behind the `cli` feature, enabled by default). The library
//! performs no terminal I/O: operations take a [`Dfm`] context plus plain
//! arguments and return report structs describing what happened; rendering
//! and prompting live in the binary.
//!
//! ```no_run
//! use dotfiles_manager::{Dfm, profiles::ActiveProfile};
//!
//! let ctx = Dfm::new()?;
//! let profile = ActiveProfile::resolve(&ctx, None);
//! let report = dotfiles_manager::backup::run(&ctx, &profile, None)?;
//! println!("{} configs backed up", report.configs.succeeded());
//! # Ok::<(), dotfiles_manager::Error>(())
//! ```

#![warn(missing_docs)]

pub mod backup;
mod context;
pub mod doctor;
pub mod edit;
pub mod encryption;
mod error;
pub mod git;
pub mod link;
pub mod profiles;
pub mod registry;
mod report;
pub mod restore;
pub mod sync;
mod utils;

/// The dfm runtime context, plus well-known file names inside the dfm root.
pub use context::{Dfm, ENCRYPTED_BUNDLE_FILE, PROFILE_CONFIG_FILE};
/// The crate's error and result types.
pub use error::{Error, Result};
/// Report types describing the outcome of registry-driven operations.
pub use report::{RegistryEntryOutcome, RegistryEntryStatus, SectionReport};
