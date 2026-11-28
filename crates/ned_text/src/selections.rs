//! Multi-selection collection managing ordered, non-overlapping selection ranges.

use ned_rope::rope;

use crate::ranges::range;

/// A selection composed of one or more non-overlapping [`range`]s.
///
/// Invariants:
/// - A selection is never empty (contains at least one primary range).
/// - Ranges are always sorted in ascending order of their start positions (`from()`).
/// - Overlapping ranges are automatically merged into a single enclosing range.
/// - Exactly one range is designated as the primary selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct selection {
    /// Ordered, non-overlapping ranges making up the selection.
    ranges: Vec<range>,
    /// Index of the primary (focused) range within `ranges`.
    primary_index: usize,
}

impl selection {
    /// Creates a selection containing a single range from anchor to head.
    #[must_use]
    pub fn single(anchor: usize, head: usize) -> Self {
        Self {
            ranges: vec![range::new(anchor, head)],
            primary_index: 0,
        }
    }

    /// Creates a selection containing a single zero-width point cursor.
    #[must_use]
    pub fn point(pos: usize) -> Self {
        Self::single(pos, pos)
    }

    /// Creates a selection from a vector of ranges, normalizing them.
    ///
    /// # Panics
    ///
    /// Panics if `ranges` is empty or if `primary_index >= ranges.len()`.
    #[must_use]
    pub fn new(ranges: Vec<range>, primary_index: usize) -> Self {
        assert!(!ranges.is_empty(), "selection cannot be empty");
        assert!(primary_index < ranges.len(), "primary_index out of bounds");

        let sel = Self {
            ranges,
            primary_index,
        };
        sel.normalize()
    }

    /// Returns the primary active range of this selection.
    #[inline]
    #[must_use]
    pub fn primary(&self) -> range {
        self.ranges[self.primary_index]
    }

    /// Returns a mutable reference to the primary active range.
    #[inline]
    #[must_use]
    pub fn primary_mut(&mut self) -> &mut range {
        &mut self.ranges[self.primary_index]
    }

    /// Returns a slice of all constituent ranges in ascending order.
    #[inline]
    #[must_use]
    pub fn ranges(&self) -> &[range] {
        &self.ranges
    }

    /// Returns the number of distinct selection ranges.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Always returns `false`, as a valid selection is guaranteed non-empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Returns the index of the primary selection range.
    #[inline]
    #[must_use]
    pub fn primary_index(&self) -> usize {
        self.primary_index
    }

    /// Sets the active primary selection index.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= self.len()`.
    pub fn set_primary_index(&mut self, idx: usize) {
        assert!(idx < self.ranges.len(), "primary index out of bounds");
        self.primary_index = idx;
    }

    /// Returns a selection retaining only the primary range.
    #[must_use]
    pub fn into_single(self) -> Self {
        Self {
            ranges: vec![self.ranges[self.primary_index]],
            primary_index: 0,
        }
    }

    /// Adds a new range, makes it primary, and normalizes the selection.
    #[must_use]
    pub fn push(mut self, r: range) -> Self {
        self.ranges.push(r);
        self.primary_index = self.ranges.len() - 1;
        self.normalize()
    }

    /// Removes the range at `index`, adjusting the primary index.
    ///
    /// # Panics
    ///
    /// Panics if `self.len() <= 1` (a selection must have at least one range)
    /// or if `index >= self.len()`.
    #[must_use]
    pub fn remove(mut self, index: usize) -> Self {
        assert!(
            self.ranges.len() > 1,
            "cannot remove the only range in selection"
        );
        self.ranges.remove(index);
        if index < self.primary_index || self.primary_index == self.ranges.len() {
            self.primary_index = self.primary_index.saturating_sub(1);
        }
        self
    }

    /// Replaces the range at `index` with `new_range` and normalizes.
    ///
    /// # Panics
    ///
    /// Panics if `index >= self.len()`.
    #[must_use]
    pub fn replace(mut self, index: usize, new_range: range) -> Self {
        self.ranges[index] = new_range;
        self.normalize()
    }

    /// Inverts the anchor and head of every range in this selection.
    #[must_use]
    pub fn flip(&self) -> Self {
        let flipped: Vec<range> = self
            .ranges
            .iter()
            .map(|r| range::new(r.head, r.anchor))
            .collect();
        Self::new(flipped, self.primary_index)
    }

    /// Ensures all selection ranges are oriented forward (`anchor <= head`).
    #[must_use]
    pub fn ensure_forward(&self) -> Self {
        let forward: Vec<range> = self
            .ranges
            .iter()
            .map(|r| {
                if r.anchor > r.head {
                    range::new(r.head, r.anchor)
                } else {
                    *r
                }
            })
            .collect();
        Self::new(forward, self.primary_index)
    }

    /// Removes the current primary selection, promoting the next surviving selection.
    ///
    /// If there is only one range, returns a clone of `self`.
    #[must_use]
    pub fn remove_primary(&self) -> Self {
        if self.ranges.len() <= 1 {
            return self.clone();
        }
        let mut new_ranges = self.ranges.clone();
        new_ranges.remove(self.primary_index);
        let new_idx = self.primary_index.min(new_ranges.len() - 1);
        Self::new(new_ranges, new_idx)
    }

    /// Cycles the active primary selection forward by `count` positions.
    pub fn cycle_primary_forward(&mut self, count: usize) {
        if !self.ranges.is_empty() {
            self.primary_index = (self.primary_index + count) % self.ranges.len();
        }
    }

    /// Cycles the active primary selection backward by `count` positions.
    pub fn cycle_primary_backward(&mut self, count: usize) {
        if !self.ranges.is_empty() {
            let step = count % self.ranges.len();
            self.primary_index =
                (self.primary_index + self.ranges.len() - step) % self.ranges.len();
        }
    }

    /// Merges consecutive adjacent ranges where `prev.to() == curr.from()`.
    #[must_use]
    pub fn merge_consecutive(mut self) -> Self {
        if self.ranges.len() < 2 {
            return self;
        }

        let mut primary = self.ranges[self.primary_index];
        let mut merged: Vec<range> = Vec::with_capacity(self.ranges.len());

        for curr in self.ranges {
            if let Some(prev) = merged.last_mut() {
                if prev.to() == curr.from() {
                    let combined = prev.merge(curr);
                    if *prev == primary || curr == primary {
                        primary = combined;
                    }
                    *prev = combined;
                    continue;
                }
            }
            merged.push(curr);
        }

        self.ranges = merged;
        self.primary_index = self.ranges.iter().position(|r| *r == primary).unwrap_or(0);
        self
    }

    /// Trims leading and trailing whitespace from each selection range.
    #[must_use]
    pub fn trim_whitespace(&self, buffer: &rope) -> Self {
        let total_chars = buffer.char_len();
        let mut new_ranges = Vec::with_capacity(self.ranges.len());

        for sel in &self.ranges {
            let from = sel.from().min(total_chars);
            let to = sel.to().min(total_chars);
            if from >= to {
                continue;
            }

            let text = buffer.slice_chars(from..to);
            let chars: Vec<char> = text.chars().collect();
            let mut start_offset = 0;
            let mut end_offset = chars.len();

            while start_offset < chars.len() && chars[start_offset].is_whitespace() {
                start_offset += 1;
            }
            while end_offset > start_offset && chars[end_offset - 1].is_whitespace() {
                end_offset -= 1;
            }

            if start_offset < end_offset {
                let start_char = from + start_offset;
                let end_char = from + end_offset;
                let r = if sel.anchor > sel.head {
                    range::new(end_char, start_char)
                } else {
                    range::new(start_char, end_char)
                };
                new_ranges.push(r);
            }
        }

        if new_ranges.is_empty() {
            Self::single(self.primary().anchor, self.primary().head)
        } else {
            let primary = self.primary();
            let idx = new_ranges
                .iter()
                .position(|r| r.overlaps(&primary))
                .unwrap_or(0);
            Self::new(new_ranges, idx)
        }
    }

    /// Splits each range in this selection across newline boundaries.
    #[must_use]
    pub fn split_on_newline(&self, buffer: &rope) -> Self {
        let mut new_ranges = Vec::with_capacity(self.ranges.len());
        let total_chars = buffer.char_len();

        for sel in &self.ranges {
            if sel.from() == sel.to() {
                new_ranges.push(*sel);
                continue;
            }

            let start_char = sel.from().min(total_chars);
            let end_char = sel.to().min(total_chars);
            if start_char >= end_char {
                new_ranges.push(*sel);
                continue;
            }

            let start_byte = buffer.char_to_byte(start_char);
            let end_byte = buffer.char_to_byte(end_char);
            let start_line = buffer.byte_to_line(start_byte);
            let end_line = buffer.byte_to_line(end_byte);

            for line_idx in start_line..=end_line {
                let l_start_byte = buffer.line_to_byte(line_idx).unwrap_or(0);
                let l_start_char = buffer.byte_to_char(l_start_byte);
                let line_str = buffer.line(line_idx).unwrap_or_default();
                let trimmed_line_chars = line_str.trim_end_matches(['\r', '\n']).chars().count();
                let l_end_char = l_start_char + trimmed_line_chars;

                let r_start = l_start_char.max(start_char);
                let r_end = l_end_char.min(end_char);
                if r_start <= r_end {
                    let r = if sel.anchor > sel.head {
                        range::new(r_end, r_start)
                    } else {
                        range::new(r_start, r_end)
                    };
                    new_ranges.push(r);
                }
            }
        }

        if new_ranges.is_empty() {
            Self::single(self.primary().anchor, self.primary().head)
        } else {
            Self::new(new_ranges, 0)
        }
    }

    /// Collapses all selection ranges to their block-cursor point positions.
    #[must_use]
    pub fn collapse(&self, buffer: &rope) -> Self {
        let new_ranges: Vec<range> = self
            .ranges
            .iter()
            .map(|r| range::point(r.cursor(buffer)))
            .collect();
        Self::new(new_ranges, self.primary_index)
    }

    /// Normalizes the selection: sorts by `from()` and merges overlapping ranges.
    #[must_use]
    pub fn normalize(mut self) -> Self {
        if self.ranges.len() < 2 {
            return self;
        }

        let mut primary = self.ranges[self.primary_index];
        self.ranges.sort_unstable_by_key(super::ranges::range::from);

        let mut deduped: Vec<range> = Vec::with_capacity(self.ranges.len());
        for r in self.ranges {
            if let Some(prev) = deduped.last_mut() {
                if prev.overlaps(&r) {
                    let merged = prev.merge(r);
                    if *prev == primary || r == primary {
                        primary = merged;
                    }
                    *prev = merged;
                } else {
                    deduped.push(r);
                }
            } else {
                deduped.push(r);
            }
        }

        self.ranges = deduped;
        self.primary_index = self.ranges.iter().position(|r| *r == primary).unwrap_or(0);
        self
    }

    /// Returns an iterator over references to the ranges in this selection.
    pub fn iter(&self) -> std::slice::Iter<'_, range> {
        self.ranges.iter()
    }
}

impl IntoIterator for selection {
    type Item = range;
    type IntoIter = std::vec::IntoIter<range>;

    fn into_iter(self) -> Self::IntoIter {
        self.ranges.into_iter()
    }
}

impl<'a> IntoIterator for &'a selection {
    type Item = &'a range;
    type IntoIter = std::slice::Iter<'a, range>;

    fn into_iter(self) -> Self::IntoIter {
        self.ranges.iter()
    }
}
