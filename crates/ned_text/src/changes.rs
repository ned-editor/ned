//! Changeset model representing atomic document edits and selection mapping.

use std::fmt;

use ned_rope::rope;

use crate::ranges::range;
use crate::selections::selection;

/// A single atomic operation in a changeset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum operation {
    /// Advances cursor position by `n` characters without modification.
    retain(usize),
    /// Deletes `n` characters starting at current position.
    delete(usize),
    /// Inserts string content at current position.
    insert(String),
}

/// Associativity rule for mapping positions through an insertion.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum assoc {
    /// Position stays before inserted text.
    before,
    /// Position shifts after inserted text.
    after,
    /// Preserves offset within sticky replacement boundaries (before side).
    before_sticky,
    /// Preserves offset within sticky replacement boundaries (after side).
    after_sticky,
}

/// Error returned when applying a changeset to a buffer of mismatched length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum change_error {
    /// Document length did not match the expected starting length.
    length_mismatch {
        /// Expected length in characters.
        expected: usize,
        /// Actual length found in document.
        found: usize,
    },
}

impl fmt::Display for change_error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::length_mismatch { expected, found } => {
                write!(
                    f,
                    "changeset length mismatch: expected {expected} chars, found {found}"
                )
            }
        }
    }
}

impl std::error::Error for change_error {}

/// Ordered sequence of retain, delete, and insert operations affecting a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct change_set {
    changes: Vec<operation>,
    len: usize,
    len_after: usize,
}

impl change_set {
    /// Creates an empty changeset for a document of initial character length `doc_len`.
    #[must_use]
    pub fn new(doc_len: usize) -> Self {
        Self {
            changes: Vec::new(),
            len: doc_len,
            len_after: doc_len,
        }
    }

    /// Retains `n` characters unchanged.
    pub fn retain(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        if let Some(operation::retain(count)) = self.changes.last_mut() {
            *count += n;
        } else {
            self.changes.push(operation::retain(n));
        }
    }

    /// Deletes `n` characters.
    pub fn delete(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        self.len_after = self.len_after.saturating_sub(n);
        if let Some(operation::delete(count)) = self.changes.last_mut() {
            *count += n;
        } else {
            self.changes.push(operation::delete(n));
        }
    }

    /// Inserts text at the current position.
    pub fn insert(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.len_after += text.chars().count();
        if let Some(operation::insert(s)) = self.changes.last_mut() {
            s.push_str(text);
        } else {
            self.changes.push(operation::insert(text.to_string()));
        }
    }

    /// Returns `true` if the changeset performs no text modifications.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
            || (self.changes.len() == 1
                && matches!(self.changes[0], operation::retain(n) if n == self.len))
    }

    /// Returns the required initial character length of the target document.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the character length of the document after applying all changes.
    #[inline]
    #[must_use]
    pub fn len_after(&self) -> usize {
        self.len_after
    }

    /// Applies all changes in this set to the mutable rope buffer.
    ///
    /// # Errors
    ///
    /// Returns [`change_error::length_mismatch`] if `buffer.char_len() != self.len()`.
    pub fn apply(&self, buffer: &mut rope) -> Result<(), change_error> {
        if buffer.char_len() != self.len {
            return Err(change_error::length_mismatch {
                expected: self.len,
                found: buffer.char_len(),
            });
        }

        let mut pos = 0;
        for op in &self.changes {
            match op {
                operation::retain(n) => {
                    pos += n;
                }
                operation::delete(n) => {
                    buffer.delete_chars(pos..pos + *n);
                }
                operation::insert(s) => {
                    buffer.insert_chars(pos, s);
                    pos += s.chars().count();
                }
            }
        }
        Ok(())
    }

    /// Maps a single character offset through the changeset according to associativity rules.
    #[must_use]
    pub fn map_pos(&self, pos: usize, associativity: assoc) -> usize {
        let mut old_pos = 0;
        let mut new_pos = 0;

        for op in &self.changes {
            match op {
                operation::retain(len) => {
                    let next_old = old_pos + len;
                    if pos < next_old || (pos == next_old && associativity == assoc::before) {
                        return new_pos + (pos - old_pos);
                    }
                    old_pos = next_old;
                    new_pos += len;
                }
                operation::delete(len) => {
                    let next_old = old_pos + len;
                    if pos <= next_old {
                        return new_pos;
                    }
                    old_pos = next_old;
                }
                operation::insert(s) => {
                    let ins_len = s.chars().count();
                    if pos == old_pos {
                        match associativity {
                            assoc::before | assoc::before_sticky => return new_pos,
                            assoc::after | assoc::after_sticky => return new_pos + ins_len,
                        }
                    }
                    new_pos += ins_len;
                }
            }
        }
        new_pos + pos.saturating_sub(old_pos)
    }

    /// Maps a selection range through this changeset.
    #[must_use]
    pub fn map_range(&self, r: range) -> range {
        use std::cmp::Ordering;
        let (anchor_assoc, head_assoc) = match r.anchor.cmp(&r.head) {
            Ordering::Equal => (assoc::after_sticky, assoc::after_sticky),
            Ordering::Less => (assoc::after_sticky, assoc::before_sticky),
            Ordering::Greater => (assoc::before_sticky, assoc::after_sticky),
        };
        let new_anchor = self.map_pos(r.anchor, anchor_assoc);
        let new_head = self.map_pos(r.head, head_assoc);
        range {
            anchor: new_anchor,
            head: new_head,
            old_visual_position: None,
        }
    }

    /// Maps a complete multi-selection through this changeset.
    #[must_use]
    pub fn map_selection(&self, sel: selection) -> selection {
        if self.is_empty() {
            return sel;
        }
        let new_ranges: Vec<range> = sel.ranges().iter().map(|&r| self.map_range(r)).collect();
        selection::new(new_ranges, sel.primary_index())
    }

    /// Inverts this changeset against the original document, producing an undo changeset.
    #[must_use]
    pub fn invert(&self, original_doc: &rope) -> Self {
        let mut inverted = Self::new(self.len_after);
        let mut pos = 0;

        for op in &self.changes {
            match op {
                operation::retain(n) => {
                    inverted.retain(*n);
                    pos += n;
                }
                operation::delete(n) => {
                    let deleted_text = original_doc.slice_chars(pos..pos + *n);
                    inverted.insert(&deleted_text);
                    pos += n;
                }
                operation::insert(s) => {
                    inverted.delete(s.chars().count());
                }
            }
        }
        inverted
    }
}
