//! Prompt state model, actions, and lifecycle management.

use ned_tui::input::events::KeyEvent;

use super::{command_doc, completion_direction};

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
    pub fn recalculate_completion(&mut self) {
        self.selection = None;
        if self.prefix != ":" {
            self.completion = Vec::new();
            return;
        }

        let trimmed = self.input.trim_start();
        let cmd_token = trimmed.split_whitespace().next().unwrap_or("");

        let mut matches: Vec<&'static command_doc> = super::COMMANDS
            .iter()
            .filter(|c| c.name.starts_with(cmd_token))
            .collect();

        // Sort alphabetically matching Helix TYPABLE_COMMAND_LIST
        matches.sort_by_key(|c| c.name);
        self.completion = matches;
    }

    /// Cycles the active completion candidate selection forward or backward.
    pub fn change_completion_selection(&mut self, direction: completion_direction) {
        if self.completion.is_empty() {
            return;
        }

        let len = self.completion.len();
        let next_idx = match direction {
            completion_direction::forward => match self.selection {
                Some(i) => (i + 1) % len,
                None => 0,
            },
            completion_direction::backward => match self.selection {
                Some(i) => {
                    if i == 0 {
                        len - 1
                    } else {
                        i - 1
                    }
                }
                None => len - 1,
            },
        };

        self.selection = Some(next_idx);
        let selected_item = self.completion[next_idx];

        // Replace command token with selected item
        let trimmed = self.input.trim_start();
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        if parts.len() > 1 {
            self.input = format!("{} {}", selected_item.name, parts[1]);
        } else {
            self.input = selected_item.name.to_string();
        }
        self.cursor = self.input.chars().count();
    }

    /// Clears the active completion selection.
    pub fn exit_selection(&mut self) {
        self.selection = None;
    }

    /// Returns the currently active documentation (from selected completion candidate or typed command).
    #[must_use]
    pub fn active_doc(&self) -> Option<(&'static str, &'static str)> {
        if let Some(idx) = self.selection {
            if let Some(cmd) = self.completion.get(idx) {
                return Some((cmd.name, cmd.doc));
            }
        }
        let token = self.input.trim_start().split_whitespace().next().unwrap_or("");
        if token.is_empty() {
            return None;
        }
        super::COMMANDS
            .iter()
            .find(|c| c.name == token)
            .map(|c| (c.name, c.doc))
    }

    /// Handles a raw keyboard event with exact Helix readline and completion behavior.
    pub fn handle_key(&mut self, ev: &KeyEvent) -> prompt_action {
        super::handle_key_internal(self, ev)
    }
}
