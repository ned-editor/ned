//! Multiline syntax state tracking for block comments and multiline string literals.

/// Syntax state at the boundary between lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum line_state {
    /// Normal code or top-level syntax.
    #[default]
    normal,
    /// Inside a C-style block comment (`/* ... */`) with nesting depth.
    in_block_comment(u8),
    /// Inside a Python/TOML triple-quoted multi-line string (`"""` or `'''`).
    in_triple_string(char),
    /// Inside a Rust raw string literal (`r###" ... "###`) with count of `#` delimiters.
    in_raw_string(u8),
}

/// Cache of line-start states allowing $O(1)$ viewport-only syntax highlighting.
#[derive(Debug, Clone, Default)]
pub struct state_cache {
    /// Stores the syntax state at the beginning of each line.
    states: Vec<line_state>,
}

impl state_cache {
    /// Creates an empty state cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
        }
    }

    /// Returns the syntax state at the start of `line_idx` (defaults to [`line_state::normal`]).
    #[must_use]
    pub fn get(&self, line_idx: usize) -> line_state {
        self.states.get(line_idx).copied().unwrap_or(line_state::normal)
    }

    /// Sets or updates the syntax state at the start of `line_idx`.
    pub fn set(&mut self, line_idx: usize, state: line_state) {
        if line_idx >= self.states.len() {
            self.states.resize(line_idx + 1, line_state::normal);
        }
        self.states[line_idx] = state;
    }

    /// Invalidates cached states starting from `line_idx` following an edit.
    pub fn invalidate_from(&mut self, line_idx: usize) {
        if line_idx < self.states.len() {
            self.states.truncate(line_idx);
        }
    }

    /// Clears all cached states.
    pub fn clear(&mut self) {
        self.states.clear();
    }

    /// Number of cached line states.
    #[must_use]
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Returns `true` if no line states are currently cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}
