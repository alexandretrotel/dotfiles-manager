mod fix;
mod report;
mod validators;

use age::secrecy::SecretString;
pub use fix::{FixedFile, fix};
pub use report::{DoctorReport, Severity, ValidationError};
use validators::ValidationSuite;

use crate::context::Dfm;
use crate::profiles::ActiveProfile;

/// Run every validator and collect findings. Encrypted backup consistency is
/// only checked when a `password` is supplied. When `include_disabled` is
/// `true`, the backup consistency check also covers disabled registry
/// entries.
pub fn validate(
    ctx: &Dfm,
    profile: &ActiveProfile,
    password: Option<&SecretString>,
    include_disabled: bool,
) -> DoctorReport {
    ValidationSuite::new(
        ctx.clone(),
        profile.clone(),
        password.cloned(),
        include_disabled,
    )
    .run_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_runs_every_validator_and_returns_one_result_each() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        let profile = ActiveProfile::common_only();

        let report = validate(&ctx, &profile, None, false);

        let results = report.results();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "Registry Files");
        assert_eq!(results[1].0, "Backup Consistency Check");
    }

    #[test]
    fn validate_surfaces_errors_from_an_unparsable_registry_file() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        std::fs::write(ctx.config_registry_path(), "not valid json {").unwrap();
        let profile = ActiveProfile::common_only();

        let report = validate(&ctx, &profile, None, false);

        assert!(report.error_count() >= 1);
    }
}
