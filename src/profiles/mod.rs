//! Named profiles layered on top of the `common` backup, and resolving
//! which layer a given source file comes from.

mod actions;
mod active;
mod config;
mod prune;
mod sources;

pub use actions::{
    COMMON_PROFILE_NAMES, CreatedProfile, DeletedProfile, SwitchProfileOutcome, create_profile,
    delete_profile, switch_profile,
};
pub use active::{
    ActiveProfile, clear_active_profile, get_active_profile_name, set_active_profile,
};
pub use config::{ProfileConfig, ProfileDefinition};
pub use prune::{OrphanedProfile, PrunedProfile, find_orphaned_profiles, prune_orphaned_profiles};
pub use sources::{ResolvedSource, SourceLayer};
