//! Highlight span definitions tracking colored text ranges.

use crate::tokens::syntax_kind;

/// A contiguous slice of text associated with a syntax highlighting token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct highlight_span {
    /// Starting character offset (inclusive).
    pub start: usize,
    /// Ending character offset (exclusive).
    pub end: usize,
    /// Syntax token kind.
    pub kind: syntax_kind,
}

impl highlight_span {
    /// Creates a new highlight span.
    #[must_use]
    pub const fn new(start: usize, end: usize, kind: syntax_kind) -> Self {
        Self { start, end, kind }
    }

    /// Returns the theme scope associated with this span.
    #[must_use]
    pub const fn scope(&self) -> &'static str {
        self.kind.as_scope()
    }

    /// Character length of this span.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns `true` if the span covers zero characters.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}
