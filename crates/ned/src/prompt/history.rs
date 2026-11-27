//! Prompt command history storage and ring navigation.

/// History storage for submitted interactive prompt commands matching Helix Prompt parity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct prompt_history {
    entries: Vec<String>,
    max_entries: usize,
    current_index: Option<usize>,
    saved_input: Option<String>,
}

impl Default for prompt_history {
    fn default() -> Self {
        Self::new(500)
    }
}

impl prompt_history {
    /// Creates a new prompt history ring with the specified capacity cap.
    #[must_use]
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries: max_entries.max(1),
            current_index: None,
            saved_input: None,
        }
    }

    /// Appends a newly submitted command to history, ignoring duplicates of the last entry.
    pub fn push(&mut self, entry: impl Into<String>) {
        let entry = entry.into();
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            return;
        }
        if self.entries.last().map(String::as_str) != Some(trimmed) {
            self.entries.push(trimmed.to_string());
            if self.entries.len() > self.max_entries {
                self.entries.remove(0);
            }
        }
        self.reset_nav();
    }

    /// Resets navigation index back to unnavigated state.
    pub fn reset_nav(&mut self) {
        self.current_index = None;
        self.saved_input = None;
    }

    /// Navigates to the previous (older) history entry, saving `current_input` if starting navigation.
    pub fn previous(&mut self, current_input: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }

        let new_idx = match self.current_index {
            None => {
                self.saved_input = Some(current_input.to_string());
                self.entries.len().saturating_sub(1)
            }
            Some(0) => 0,
            Some(i) => i.saturating_sub(1),
        };

        self.current_index = Some(new_idx);
        self.entries.get(new_idx).map(String::as_str)
    }

    /// Navigates to the next (newer) history entry, returning saved input when passing the latest entry.
    pub fn next(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }

        match self.current_index {
            None => None,
            Some(i) if i + 1 >= self.entries.len() => {
                self.current_index = None;
                self.saved_input.as_deref()
            }
            Some(i) => {
                let next_idx = i + 1;
                self.current_index = Some(next_idx);
                self.entries.get(next_idx).map(String::as_str)
            }
        }
    }

    /// Returns the total number of entries stored in history.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if history contains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
