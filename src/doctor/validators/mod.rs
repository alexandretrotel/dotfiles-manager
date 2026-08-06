mod backup_consistency;
mod registry_files;

use age::secrecy::SecretString;

use backup_consistency::BackupConsistencyValidator;
use registry_files::RegistryFilesValidator;

use crate::context::Dfm;
use crate::doctor::report::{DoctorReport, ValidationError};
use crate::profiles::ActiveProfile;

/// One check run as part of `dfm doctor`.
pub(crate) trait Validator {
    /// Display name for this check, shown in the report.
    fn name(&self) -> &str;
    /// Run the check and return its findings.
    fn validate(&self) -> Vec<ValidationError>;
}

/// The full set of `dfm doctor` checks, run in a fixed order.
pub(crate) struct ValidationSuite {
    validators: Vec<Box<dyn Validator>>,
}

impl ValidationSuite {
    /// Build the fixed list of validators, in the order they'll run.
    /// `include_disabled` also checks disabled registry entries in the
    /// backup consistency check.
    pub(crate) fn new(
        ctx: Dfm,
        profile: ActiveProfile,
        password: Option<SecretString>,
        include_disabled: bool,
    ) -> Self {
        let validators: Vec<Box<dyn Validator>> = vec![
            Box::new(RegistryFilesValidator::new(ctx.clone())),
            Box::new(BackupConsistencyValidator::new(
                ctx,
                profile,
                password,
                include_disabled,
            )),
        ];
        Self { validators }
    }

    /// Run every validator and collect their findings into one report.
    pub(crate) fn run_all(&self) -> DoctorReport {
        let mut report = DoctorReport::default();
        for validator in &self.validators {
            let errors = validator.validate();
            report.add_result(validator.name(), errors);
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Dfm;

    #[test]
    fn run_all_produces_one_result_per_validator_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        let profile = ActiveProfile::common_only();

        let suite = ValidationSuite::new(ctx, profile, None, false);
        let report = suite.run_all();

        let results = report.results();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "Registry Files");
        assert_eq!(results[1].0, "Backup Consistency Check");
    }

    #[test]
    fn new_builds_suite_with_two_validators() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        let profile = ActiveProfile::common_only();

        let suite = ValidationSuite::new(ctx, profile, None, false);

        assert_eq!(suite.validators.len(), 2);
    }
}
