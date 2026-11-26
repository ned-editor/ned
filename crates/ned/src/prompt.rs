//! Interactive command prompt and search bar for bottom line input.

use ned_tui::input::events::{KeyCode, KeyEvent, KeyModifier};

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

/// State of an active bottom-line interactive prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct prompt_state {
    /// Prompt prefix symbol (e.g. `":"` or `"/"`).
    pub prefix: String,
    /// User input text buffer.
    pub input: String,
    /// Cursor character index within `input`.
    pub cursor: usize,
}

impl prompt_state {
    /// Creates a new prompt with the specified prefix symbol.
    #[must_use]
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            input: String::new(),
            cursor: 0,
        }
    }

    /// Handles a raw keyboard event while the prompt is focused.
    pub fn handle_key(&mut self, ev: &KeyEvent) -> prompt_action {
        let ctrl = ev.modifiers.contains(&KeyModifier::Control);

        match ev.keycode {
            KeyCode::Esc => prompt_action::cancel,
            KeyCode::Enter => prompt_action::submit(self.input.clone()),
            KeyCode::Backspace => {
                if ctrl {
                    // Delete word backward
                    while self.cursor > 0 && self.input.chars().nth(self.cursor - 1) == Some(' ') {
                        self.cursor -= 1;
                        self.input.remove(self.cursor);
                    }
                    while self.cursor > 0 && self.input.chars().nth(self.cursor - 1) != Some(' ') {
                        self.cursor -= 1;
                        self.input.remove(self.cursor);
                    }
                    prompt_action::none
                } else if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                    prompt_action::none
                } else {
                    // Backspace on empty input closes prompt matching Helix
                    prompt_action::cancel
                }
            }
            KeyCode::Delete => {
                if self.cursor < self.input.chars().count() {
                    self.input.remove(self.cursor);
                }
                prompt_action::none
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                prompt_action::none
            }
            KeyCode::Right => {
                if self.cursor < self.input.chars().count() {
                    self.cursor += 1;
                }
                prompt_action::none
            }
            KeyCode::Home => {
                self.cursor = 0;
                prompt_action::none
            }
            KeyCode::End => {
                self.cursor = self.input.chars().count();
                prompt_action::none
            }
            KeyCode::Char(ch) => {
                if ctrl {
                    match ch {
                        'w' => {
                            // Delete word backward
                            while self.cursor > 0
                                && self.input.chars().nth(self.cursor - 1) == Some(' ')
                            {
                                self.cursor -= 1;
                                self.input.remove(self.cursor);
                            }
                            while self.cursor > 0
                                && self.input.chars().nth(self.cursor - 1) != Some(' ')
                            {
                                self.cursor -= 1;
                                self.input.remove(self.cursor);
                            }
                            prompt_action::none
                        }
                        'u' => {
                            // Clear line before cursor
                            self.input.drain(..self.cursor);
                            self.cursor = 0;
                            prompt_action::none
                        }
                        'c' => prompt_action::cancel,
                        _ => prompt_action::none,
                    }
                } else {
                    self.input.insert(self.cursor, ch);
                    self.cursor += 1;
                    prompt_action::none
                }
            }
            _ => prompt_action::none,
        }
    }
}
