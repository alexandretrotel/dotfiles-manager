/// Outcome of processing a single registry entry during backup or restore.
#[derive(Debug, Clone)]
pub enum RegistryEntryStatus {
    /// Entry was processed; `note` carries any extra information (e.g. a
    /// symlink that was converted to a real file along the way).
    Done { note: Option<String> },
    /// Entry was skipped; `reason` explains why.
    Skipped { reason: String },
}

/// Result for one registry entry processed during a backup or restore run,
/// pairing the entry's identity with its [`RegistryEntryStatus`].
#[derive(Debug, Clone)]
pub struct RegistryEntryOutcome {
    /// Registry entry id.
    pub id: String,
    /// Human-oriented label (usually the backup path).
    pub label: String,
    pub status: RegistryEntryStatus,
}

impl RegistryEntryOutcome {
    /// Build a [`RegistryEntryStatus::Done`] outcome with no note.
    pub(crate) fn done(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            status: RegistryEntryStatus::Done { note: None },
        }
    }

    /// Build a [`RegistryEntryStatus::Done`] outcome carrying an informational note.
    pub(crate) fn done_with_note(
        id: impl Into<String>,
        label: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            status: RegistryEntryStatus::Done {
                note: Some(note.into()),
            },
        }
    }

    /// Build a [`RegistryEntryStatus::Skipped`] outcome with the given reason.
    pub(crate) fn skipped(
        id: impl Into<String>,
        label: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            status: RegistryEntryStatus::Skipped {
                reason: reason.into(),
            },
        }
    }

    /// Whether this entry completed successfully (regardless of any note).
    pub(crate) fn is_done(&self) -> bool {
        matches!(self.status, RegistryEntryStatus::Done { .. })
    }
}

/// Per-section results of a backup or restore run.
#[derive(Debug, Clone, Default)]
pub struct SectionReport {
    pub outcomes: Vec<RegistryEntryOutcome>,
    /// Non-fatal warnings that apply to the section as a whole (e.g. an
    /// encrypted bundle that could not be decrypted).
    pub warnings: Vec<String>,
}

impl SectionReport {
    /// Number of entries that completed successfully.
    pub fn succeeded(&self) -> usize {
        self.outcomes.iter().filter(|o| o.is_done()).count()
    }

    /// Number of entries that were skipped.
    pub fn skipped(&self) -> usize {
        self.outcomes.len() - self.succeeded()
    }

    /// Whether the section has no outcomes and no warnings.
    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty() && self.warnings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn done_outcome_has_no_note_and_is_done() {
        let outcome = RegistryEntryOutcome::done("id-1", "~/.zshrc");
        assert_eq!(outcome.id, "id-1");
        assert_eq!(outcome.label, "~/.zshrc");
        assert!(outcome.is_done());
        assert!(matches!(
            outcome.status,
            RegistryEntryStatus::Done { note: None }
        ));
    }

    #[test]
    fn done_with_note_outcome_carries_the_note() {
        let outcome = RegistryEntryOutcome::done_with_note("id-1", "~/.zshrc", "converted symlink");
        assert!(outcome.is_done());
        match outcome.status {
            RegistryEntryStatus::Done { note: Some(note) } => assert_eq!(note, "converted symlink"),
            other => panic!("expected Done with note, got {other:?}"),
        }
    }

    #[test]
    fn skipped_outcome_is_not_done_and_carries_reason() {
        let outcome = RegistryEntryOutcome::skipped("id-2", "~/.vimrc", "already up to date");
        assert!(!outcome.is_done());
        match outcome.status {
            RegistryEntryStatus::Skipped { reason } => assert_eq!(reason, "already up to date"),
            other => panic!("expected Skipped, got {other:?}"),
        }
    }

    #[test]
    fn empty_section_report_is_empty() {
        assert!(SectionReport::default().is_empty());
    }

    #[test]
    fn section_report_with_outcomes_is_not_empty() {
        let mut report = SectionReport::default();
        report
            .outcomes
            .push(RegistryEntryOutcome::done("id-1", "a"));
        assert!(!report.is_empty());
    }

    #[test]
    fn section_report_with_only_warnings_is_not_empty() {
        let mut report = SectionReport::default();
        report.warnings.push("could not decrypt bundle".to_string());
        assert!(!report.is_empty());
    }

    #[test]
    fn succeeded_and_skipped_counts_reflect_outcome_mix() {
        let mut report = SectionReport::default();
        report
            .outcomes
            .push(RegistryEntryOutcome::done("id-1", "a"));
        report
            .outcomes
            .push(RegistryEntryOutcome::done_with_note("id-2", "b", "note"));
        report
            .outcomes
            .push(RegistryEntryOutcome::skipped("id-3", "c", "reason"));

        assert_eq!(report.succeeded(), 2);
        assert_eq!(report.skipped(), 1);
    }
}
