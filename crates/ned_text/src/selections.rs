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
    ranges: Vec<range>,
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
        assert!(
            primary_index < ranges.len(),
            "primary_index out of bounds"
        );

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
        assert!(self.ranges.len() > 1, "cannot remove the only range in selection");
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
        self.ranges.sort_unstable_by_key(|r| r.from());

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
        self.primary_index = self
            .ranges
            .iter()
            .position(|r| *r == primary)
            .unwrap_or(0);
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
