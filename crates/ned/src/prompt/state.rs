//! Prompt state model, actions, and lifecycle management.

use ned_tui::input::events::KeyEvent;

use super::completion::{command_doc, completion_direction};

/// Result of processing a key event in prompt mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum prompt_action {
    /// No prompt state change or pending input.
    none,
    /// Prompt was cancelled (e.g. Esc pressed).
    cancel,
    /// Command or query was submitted with the given input.
    submit(String),
}

/// State of an active bottom-line interactive prompt matching Helix Prompt parity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct prompt_state {
    /// Prompt prefix symbol (e.g. `":"` or `"/"`).
    pub prefix: String,
    /// User input text buffer.
    pub input: String,
    /// Cursor character index within `input`.
    pub cursor: usize,
    /// Matching candidate items.
    pub completion: Vec<&'static command_doc>,
    /// Selected completion candidate index if active.
    pub selection: Option<usize>,
}

impl prompt_state {
    /// Creates a new prompt with the specified prefix symbol and calculates initial completions.
    #[must_use]
    pub fn new(prefix: impl Into<String>) -> Self {
        let pfx = prefix.into();
        let mut prompt = Self {
            prefix: pfx,
            input: String::new(),
            cursor: 0,
            completion: Vec::new(),
            selection: None,
        };
        prompt.recalculate_completion();
        prompt
    }

    /// Recalculates matching completions based on the currently typed command prefix.
    #[inline]
    pub fn recalculate_completion(&mut self) {
        super::completion::recalculate_completion(self);
    }

    /// Cycles the active completion candidate selection forward or backward.
    #[inline]
    pub fn change_completion_selection(&mut self, direction: completion_direction) {
        super::completion::change_completion_selection(self, direction);
    }

    /// Clears the active completion selection.
    #[inline]
    pub fn exit_selection(&mut self) {
        super::completion::exit_selection(self);
    }

    /// Returns the currently active documentation (from selected completion candidate or typed command).
    #[inline]
    #[must_use]
    pub fn active_doc(&self) -> Option<(&'static str, &'static str)> {
        super::completion::active_doc(self)
    }

    /// Handles a raw keyboard event with exact Helix readline and completion behavior.
    #[inline]
    pub fn handle_key(&mut self, ev: &KeyEvent) -> prompt_action {
        super::editing::handle_key(self, ev)
    }
}
