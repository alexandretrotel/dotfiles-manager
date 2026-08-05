//! Named profiles layered on top of the `common` backup, and resolving
//! which layer a given source file comes from.

mod active;
mod config;
mod ops;
mod sources;

pub use active::{
    ActiveProfile, clear_active_profile, get_active_profile_name, set_active_profile,
};
pub use config::{ProfileConfig, ProfileDefinition};
pub use ops::{
    COMMON_PROFILE_NAMES, CreatedProfile, DeletedProfile, SwitchProfileOutcome, create_profile,
    delete_profile, switch_profile,
};
pub use sources::{ResolvedSource, SourceLayer};
