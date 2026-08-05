/// Outcome of processing a single item during backup or restore.
#[derive(Debug, Clone)]
pub enum ItemStatus {
    /// Item was processed; `note` carries any extra information (e.g. a
    /// symlink that was converted to a real file along the way).
    Done { note: Option<String> },
    /// Item was skipped; `reason` explains why.
    Skipped { reason: String },
}

/// Result for one registry entry processed during a backup or restore run,
/// pairing the entry's identity with its [`ItemStatus`].
#[derive(Debug, Clone)]
pub struct ItemOutcome {
    /// Registry entry id.
    pub id: String,
    /// Human-oriented label (usually the source path).
    pub label: String,
    pub status: ItemStatus,
}

impl ItemOutcome {
    /// Build a [`ItemStatus::Done`] outcome with no note.
    pub(crate) fn done(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            status: ItemStatus::Done { note: None },
        }
    }

    /// Build a [`ItemStatus::Done`] outcome carrying an informational note.
    pub(crate) fn done_with_note(
        id: impl Into<String>,
        label: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            status: ItemStatus::Done {
                note: Some(note.into()),
            },
        }
    }

    /// Build a [`ItemStatus::Skipped`] outcome with the given reason.
    pub(crate) fn skipped(
        id: impl Into<String>,
        label: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            status: ItemStatus::Skipped {
                reason: reason.into(),
            },
        }
    }

    /// Whether this item completed successfully (regardless of any note).
    pub fn is_done(&self) -> bool {
        matches!(self.status, ItemStatus::Done { .. })
    }
}

/// Per-section results of a backup or restore run.
#[derive(Debug, Clone, Default)]
pub struct SectionReport {
    pub outcomes: Vec<ItemOutcome>,
    /// Non-fatal warnings that apply to the section as a whole (e.g. an
    /// encrypted bundle that could not be decrypted before falling back to
    /// per-file backups).
    pub warnings: Vec<String>,
}

impl SectionReport {
    /// Number of items that completed successfully.
    pub fn succeeded(&self) -> usize {
        self.outcomes.iter().filter(|o| o.is_done()).count()
    }

    /// Number of items that were skipped.
    pub fn skipped(&self) -> usize {
        self.outcomes.len() - self.succeeded()
    }

    /// Whether the section has no outcomes and no warnings.
    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty() && self.warnings.is_empty()
    }
}
