//! Directed selection range representing an anchor and head cursor position.

use std::cmp::Ordering;

use ned_rope::rope;

use crate::graphemes::{
    ensure_grapheme_boundary_next, ensure_grapheme_boundary_prev, next_grapheme_boundary,
    prev_grapheme_boundary,
};

/// Direction of a selection range relative to document order.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum direction {
    /// Range extends forward (head >= anchor).
    forward,
    /// Range extends backward (head < anchor).
    backward,
}

/// A directed selection range consisting of an anchor and head position.
///
/// In selection-first editors like Helix and Kakoune, selections are directed.
/// The `head` is the cursor end that moves when extending a selection, while the
/// `anchor` remains stationary.
///
/// `old_visual_position` stores the (`visual_row`, `visual_col`) for sticky column
/// retention during vertical cursor movements across lines of differing lengths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct range {
    /// Stationary end of the range.
    pub anchor: usize,
    /// Active cursor end of the range.
    pub head: usize,
    /// Saved (`visual_row`, `visual_column`) for vertical navigation.
    pub old_visual_position: Option<(u32, u32)>,
}

impl range {
    /// Creates a new range from anchor and head positions.
    #[must_use]
    pub fn new(anchor: usize, head: usize) -> Self {
        Self {
            anchor,
            head,
            old_visual_position: None,
        }
    }

    /// Creates a zero-width point range where anchor equals head.
    #[must_use]
    pub fn point(pos: usize) -> Self {
        Self::new(pos, pos)
    }

    /// Start offset of the range (minimum of anchor and head).
    #[inline]
    #[must_use]
    pub fn from(&self) -> usize {
        self.anchor.min(self.head)
    }

    /// End offset of the range (maximum of anchor and head).
    #[inline]
    #[must_use]
    pub fn to(&self) -> usize {
        self.anchor.max(self.head)
    }

    /// Total character length of the range.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.to() - self.from()
    }

    /// Returns `true` when anchor and head are at the identical offset.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Returns the orientation direction of the selection range.
    #[inline]
    #[must_use]
    pub fn direction(&self) -> direction {
        if self.head < self.anchor {
            direction::backward
        } else {
            direction::forward
        }
    }

    /// Inverts the selection direction by swapping anchor and head.
    #[must_use]
    pub fn flip(&self) -> Self {
        Self {
            anchor: self.head,
            head: self.anchor,
            old_visual_position: self.old_visual_position,
        }
    }

    /// Returns the range oriented in the requested direction.
    #[must_use]
    pub fn with_direction(self, dir: direction) -> Self {
        if self.direction() == dir {
            self
        } else {
            self.flip()
        }
    }

    /// Checks if this range overlaps with another range.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.from() == other.from() || (self.to() > other.from() && other.to() > self.from())
    }

    /// Checks if this range completely contains another range.
    #[inline]
    #[must_use]
    pub fn contains_range(&self, other: &Self) -> bool {
        self.from() <= other.from() && self.to() >= other.to()
    }

    /// Checks if this range contains the character offset.
    #[inline]
    #[must_use]
    pub fn contains(&self, pos: usize) -> bool {
        self.from() <= pos && pos < self.to()
    }

    /// Extends the range boundaries to encompass at least `[from, to]`.
    #[must_use]
    pub fn extend(&self, from: usize, to: usize) -> Self {
        if self.anchor <= self.head {
            Self {
                anchor: self.anchor.min(from),
                head: self.head.max(to),
                old_visual_position: None,
            }
        } else {
            Self {
                anchor: self.anchor.max(to),
                head: self.head.min(from),
                old_visual_position: None,
            }
        }
    }

    /// Merges two ranges into a single enclosing range, preserving direction if shared.
    #[must_use]
    pub fn merge(&self, other: Self) -> Self {
        if self.anchor > self.head && other.anchor > other.head {
            Self {
                anchor: self.anchor.max(other.anchor),
                head: self.head.min(other.head),
                old_visual_position: None,
            }
        } else {
            Self {
                anchor: self.from().min(other.from()),
                head: self.to().max(other.to()),
                old_visual_position: None,
            }
        }
    }

    /// Gets the left-side character position of the block cursor.
    ///
    /// For forward selections, the block cursor sits on the final grapheme before `head`.
    /// For backward selections, the cursor sits at `head`.
    #[must_use]
    pub fn cursor(&self, buffer: &rope) -> usize {
        if self.head > self.anchor {
            prev_grapheme_boundary(buffer, self.head)
        } else {
            self.head
        }
    }

    /// Sets the left side of the block cursor at `char_idx`, optionally extending the range.
    #[must_use]
    pub fn put_cursor(self, buffer: &rope, char_idx: usize, extend: bool) -> Self {
        if extend {
            let anchor = if self.head >= self.anchor && char_idx < self.anchor {
                next_grapheme_boundary(buffer, self.anchor)
            } else if self.head < self.anchor && char_idx >= self.anchor {
                prev_grapheme_boundary(buffer, self.anchor)
            } else {
                self.anchor
            };

            if anchor <= char_idx {
                Self::new(anchor, next_grapheme_boundary(buffer, char_idx))
            } else {
                Self::new(anchor, char_idx)
            }
        } else {
            Self::point(char_idx)
        }
    }

    /// Returns the 0-based logical line number containing the cursor.
    #[must_use]
    pub fn cursor_line(&self, buffer: &rope) -> usize {
        let cursor_char = self.cursor(buffer);
        let byte_offset = buffer.char_to_byte(cursor_char);
        buffer.byte_to_line(byte_offset)
    }

    /// Returns the (`start_line`, `end_line`) inclusive lines overlapped by this range.
    #[must_use]
    pub fn line_range(&self, buffer: &rope) -> (usize, usize) {
        let from = self.from();
        let to = if self.is_empty() {
            self.to()
        } else {
            prev_grapheme_boundary(buffer, self.to()).max(from)
        };

        let start_line = buffer.byte_to_line(buffer.char_to_byte(from));
        let end_line = buffer.byte_to_line(buffer.char_to_byte(to));
        (start_line, end_line)
    }

    /// Ensures the range has a minimum width of at least 1 grapheme.
    #[must_use]
    pub fn min_width_1(&self, buffer: &rope) -> Self {
        if self.anchor == self.head {
            Self {
                anchor: self.anchor,
                head: next_grapheme_boundary(buffer, self.head),
                old_visual_position: self.old_visual_position,
            }
        } else {
            *self
        }
    }

    /// Aligns both anchor and head to grapheme boundaries.
    #[must_use]
    pub fn grapheme_aligned(&self, buffer: &rope) -> Self {
        let (new_anchor, new_head) = match self.anchor.cmp(&self.head) {
            Ordering::Equal => {
                let pos = ensure_grapheme_boundary_prev(buffer, self.anchor);
                (pos, pos)
            }
            Ordering::Less => (
                ensure_grapheme_boundary_prev(buffer, self.anchor),
                ensure_grapheme_boundary_next(buffer, self.head),
            ),
            Ordering::Greater => (
                ensure_grapheme_boundary_next(buffer, self.anchor),
                ensure_grapheme_boundary_prev(buffer, self.head),
            ),
        };

        Self {
            anchor: new_anchor,
            head: new_head,
            old_visual_position: if new_anchor == self.anchor {
                self.old_visual_position
            } else {
                None
            },
        }
    }

    /// Extracts the text covered by this range from the rope buffer.
    #[must_use]
    pub fn slice(&self, buffer: &rope) -> String {
        buffer.slice_chars(self.from()..self.to())
    }
}

impl From<(usize, usize)> for range {
    fn from((anchor, head): (usize, usize)) -> Self {
        Self::new(anchor, head)
    }
}
