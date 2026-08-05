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
    pub(crate) fn done(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            status: ItemStatus::Done { note: None },
        }
    }

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
    pub fn succeeded(&self) -> usize {
        self.outcomes.iter().filter(|o| o.is_done()).count()
    }

    pub fn skipped(&self) -> usize {
        self.outcomes.len() - self.succeeded()
    }

    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty() && self.warnings.is_empty()
    }
}
