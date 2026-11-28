//! Undo and redo revision history tracking using reversible changesets and selections.

use ned_rope::rope;
use ned_text::{change_set, selection};

/// A single atomic point in the document edit history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct history_entry {
    /// Forward changes applied during this revision.
    pub changes: change_set,
    /// Inversion changes that undo this revision.
    pub inversion: change_set,
    /// Cursor and selection state immediately prior to applying changes.
    pub selection_before: selection,
    /// Cursor and selection state immediately after applying changes.
    pub selection_after: selection,
}

/// Linear undo/redo history manager for a document buffer.
#[derive(Debug, Default, Clone)]
pub struct history {
    /// Recorded edit revisions, oldest first.
    entries: Vec<history_entry>,
    /// Cursor index into `entries` for undo/redo position.
    cursor: usize,
}

impl history {
    /// Creates a new empty history.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
        }
    }

    /// Commits a new change revision, discarding any redo branch beyond current position.
    pub fn commit(
        &mut self,
        changes: change_set,
        before_doc: &rope,
        selection_before: selection,
        selection_after: selection,
    ) {
        if changes.is_empty() {
            return;
        }

        // Truncate redo branches
        self.entries.truncate(self.cursor);

        let inversion = changes.invert(before_doc);
        self.entries.push(history_entry {
            changes,
            inversion,
            selection_before,
            selection_after,
        });
        self.cursor = self.entries.len();
    }

    /// Reverts the most recent revision, applying its inversion to `doc`.
    ///
    /// Returns the restored selection state if successful.
    pub fn undo(&mut self, doc: &mut rope) -> Option<selection> {
        if self.cursor == 0 {
            return None;
        }

        self.cursor -= 1;
        let entry = &self.entries[self.cursor];
        if entry.inversion.apply(doc).is_ok() {
            Some(entry.selection_before.clone())
        } else {
            // Restore cursor on failure
            self.cursor += 1;
            None
        }
    }

    /// Reapplies the next undone revision to `doc`.
    ///
    /// Returns the restored selection state if successful.
    pub fn redo(&mut self, doc: &mut rope) -> Option<selection> {
        if self.cursor >= self.entries.len() {
            return None;
        }

        let entry = &self.entries[self.cursor];
        if entry.changes.apply(doc).is_ok() {
            self.cursor += 1;
            Some(entry.selection_after.clone())
        } else {
            None
        }
    }

    /// Returns the current revision index (equal to number of undos available).
    #[inline]
    #[must_use]
    pub fn current_revision(&self) -> usize {
        self.cursor
    }

    /// Returns `true` if no history revisions have been recorded.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the total number of committed revisions.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
