mod backup_consistency;
mod layer_resolution;
mod registry_files;

use age::secrecy::SecretString;

use backup_consistency::BackupConsistencyValidator;
use layer_resolution::LayerResolutionValidator;
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
    pub(crate) fn new(ctx: Dfm, profile: ActiveProfile, password: Option<SecretString>) -> Self {
        let validators: Vec<Box<dyn Validator>> = vec![
            Box::new(RegistryFilesValidator::new(ctx.clone())),
            Box::new(LayerResolutionValidator::new(ctx.clone(), profile.clone())),
            Box::new(BackupConsistencyValidator::new(ctx, profile, password)),
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
