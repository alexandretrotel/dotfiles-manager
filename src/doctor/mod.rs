mod report;
mod validators;

use age::secrecy::SecretString;
pub use report::{DoctorReport, Severity, ValidationError};
use validators::ValidationSuite;

use crate::context::Dfm;
use crate::profiles::ActiveProfile;

/// Run every validator and collect findings. Encrypted backup consistency is
/// only checked when a `password` is supplied.
pub fn validate(
    ctx: &Dfm,
    profile: &ActiveProfile,
    password: Option<&SecretString>,
) -> DoctorReport {
    ValidationSuite::new(ctx.clone(), profile.clone(), password.cloned()).run_all()
}
