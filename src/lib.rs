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

pub mod backup;
mod context;
pub mod doctor;
pub mod encryption;
mod error;
pub mod git;
pub mod keyring;
pub mod profiles;
pub mod registry;
mod report;
pub mod restore;
pub mod sync;
mod util;

pub use context::{Dotfm, ENCRYPTED_BUNDLE_FILE, PROFILE_CONFIG_FILE};
pub use error::{Error, Result};
pub use report::{ItemOutcome, ItemStatus, SectionReport};
