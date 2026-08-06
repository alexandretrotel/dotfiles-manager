/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A finding that fails the overall validation run.
    Error,
    /// A finding that is reported but does not fail the run.
    Warning,
    /// A purely informational finding.
    Info,
}

/// A single finding produced by a validator.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// How severe this finding is.
    pub severity: Severity,
    /// Human-readable description of the finding.
    pub message: String,
    /// Optional suggested remediation for this finding.
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
    /// Findings recorded by each validator, in the order they ran.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_constructor_sets_error_severity_and_no_fix() {
        let err = ValidationError::error("something broke");
        assert_eq!(err.severity, Severity::Error);
        assert_eq!(err.message, "something broke");
        assert!(err.fix_suggestion.is_none());
    }

    #[test]
    fn warning_constructor_sets_warning_severity() {
        let err = ValidationError::warning("drift detected");
        assert_eq!(err.severity, Severity::Warning);
        assert_eq!(err.message, "drift detected");
    }

    #[test]
    fn info_constructor_sets_info_severity() {
        let err = ValidationError::info("just fyi");
        assert_eq!(err.severity, Severity::Info);
        assert_eq!(err.message, "just fyi");
    }

    #[test]
    fn with_fix_attaches_suggestion() {
        let err = ValidationError::warning("drift detected").with_fix("run dfm backup");
        assert_eq!(err.fix_suggestion.as_deref(), Some("run dfm backup"));
    }

    #[test]
    fn add_result_stores_findings_in_run_order() {
        let mut report = DoctorReport::default();
        report.add_result("Registry Files", vec![ValidationError::info("a")]);
        report.add_result(
            "Backup Consistency Check",
            vec![ValidationError::error("b")],
        );

        let results = report.results();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "Registry Files");
        assert_eq!(results[1].0, "Backup Consistency Check");
    }

    #[test]
    fn error_count_counts_only_error_severity_across_validators() {
        let mut report = DoctorReport::default();
        report.add_result(
            "A",
            vec![ValidationError::error("e1"), ValidationError::warning("w1")],
        );
        report.add_result("B", vec![ValidationError::error("e2")]);

        assert_eq!(report.error_count(), 2);
    }

    #[test]
    fn warning_count_counts_only_warning_severity_across_validators() {
        let mut report = DoctorReport::default();
        report.add_result(
            "A",
            vec![ValidationError::warning("w1"), ValidationError::info("i1")],
        );
        report.add_result("B", vec![ValidationError::warning("w2")]);

        assert_eq!(report.warning_count(), 2);
    }

    #[test]
    fn default_report_has_no_results_and_zero_counts() {
        let report = DoctorReport::default();
        assert!(report.results().is_empty());
        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 0);
    }
}
