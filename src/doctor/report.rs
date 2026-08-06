/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub severity: Severity,
    pub message: String,
    pub fix_suggestion: Option<String>,
}

impl ValidationError {
    /// A finding that fails the overall validation run.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            fix_suggestion: None,
        }
    }

    /// A finding that is reported but does not fail the run.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            fix_suggestion: None,
        }
    }

    /// A purely informational finding.
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            message: message.into(),
            fix_suggestion: None,
        }
    }

    /// Attach a suggested remediation to this finding.
    pub fn with_fix(mut self, suggestion: impl Into<String>) -> Self {
        self.fix_suggestion = Some(suggestion.into());
        self
    }
}

/// Findings of a full validation run, grouped by validator.
#[derive(Debug, Clone, Default)]
pub struct DoctorReport {
    results: Vec<(String, Vec<ValidationError>)>,
}

impl DoctorReport {
    /// Record one validator's findings.
    pub(crate) fn add_result(&mut self, validator_name: &str, errors: Vec<ValidationError>) {
        self.results.push((validator_name.to_string(), errors));
    }

    /// `(validator name, findings)` pairs, in run order.
    pub fn results(&self) -> &[(String, Vec<ValidationError>)] {
        &self.results
    }

    /// Count findings across all validators at a given severity.
    fn count_by_severity(&self, severity: Severity) -> usize {
        self.results
            .iter()
            .flat_map(|(_, errors)| errors.iter())
            .filter(|e| e.severity == severity)
            .count()
    }

    /// Total number of [`Severity::Error`] findings.
    pub fn error_count(&self) -> usize {
        self.count_by_severity(Severity::Error)
    }

    /// Total number of [`Severity::Warning`] findings.
    pub fn warning_count(&self) -> usize {
        self.count_by_severity(Severity::Warning)
    }
}
