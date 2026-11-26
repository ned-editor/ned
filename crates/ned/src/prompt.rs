//! Interactive command prompt and search bar for bottom line input with auto-completion.

use ned_tui::input::events::{KeyCode, KeyEvent, KeyModifier};

/// Documentation for an editor ex-command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct command_doc {
    /// Command name.
    pub name: &'static str,
    /// Human-readable explanation.
    pub doc: &'static str,
}

/// Available ex-commands with inline help documentation.
pub const COMMANDS: &[command_doc] = &[
    command_doc {
        name: "w",
        doc: "Save current buffer to disk (:w [file])",
    },
    command_doc {
        name: "write",
        doc: "Save current buffer to disk",
    },
    command_doc {
        name: "q",
        doc: "Quit editor (:q)",
    },
    command_doc {
        name: "quit",
        doc: "Quit editor",
    },
    command_doc {
        name: "wq",
        doc: "Save buffer to disk and quit (:wq)",
    },
    command_doc {
        name: "x",
        doc: "Save buffer to disk and quit (:x)",
    },
    command_doc {
        name: "q!",
        doc: "Force quit discarding unsaved changes (:q!)",
    },
    command_doc {
        name: "quit!",
        doc: "Force quit discarding unsaved changes",
    },
    command_doc {
        name: "o",
        doc: "Open a file into a buffer (:o <path>)",
    },
    command_doc {
        name: "open",
        doc: "Open a file into a buffer (:open <path>)",
    },
    command_doc {
        name: "e",
        doc: "Edit a file (:e <path>)",
    },
    command_doc {
        name: "theme",
        doc: "Change the active color theme (:theme <name>)",
    },
    command_doc {
        name: "set",
        doc: "Set editor option (:set <key> <val>)",
    },
];

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
    /// Selected completion candidate index if active.
    pub completion_idx: Option<usize>,
}

impl prompt_state {
    /// Creates a new prompt with the specified prefix symbol.
    #[must_use]
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            input: String::new(),
            cursor: 0,
            completion_idx: None,
        }
    }

    /// Returns matching command completions based on the current input prefix.
    #[must_use]
    pub fn completions(&self) -> Vec<&'static command_doc> {
        if self.prefix != ":" {
            return Vec::new();
        }
        let trimmed = self.input.trim_start();
        let cmd_token = trimmed.split_whitespace().next().unwrap_or("");
        COMMANDS
            .iter()
            .filter(|c| c.name.starts_with(cmd_token))
            .collect()
    }

    /// Handles a raw keyboard event while the prompt is focused.
    pub fn handle_key(&mut self, ev: &KeyEvent) -> prompt_action {
        let ctrl = ev.modifiers.contains(&KeyModifier::Control);

        match ev.keycode {
            KeyCode::Esc => prompt_action::cancel,
            KeyCode::Enter => prompt_action::submit(self.input.clone()),
            KeyCode::Tab => {
                let matches = self.completions();
                if !matches.is_empty() {
                    let next_idx = match self.completion_idx {
                        Some(i) => (i + 1) % matches.len(),
                        None => 0,
                    };
                    self.completion_idx = Some(next_idx);
                    self.input = format!("{} ", matches[next_idx].name);
                    self.cursor = self.input.chars().count();
                }
                prompt_action::none
            }
            KeyCode::BackTab => {
                let matches = self.completions();
                if !matches.is_empty() {
                    let prev_idx = match self.completion_idx {
                        Some(i) => {
                            if i == 0 {
                                matches.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => matches.len() - 1,
                    };
                    self.completion_idx = Some(prev_idx);
                    self.input = format!("{} ", matches[prev_idx].name);
                    self.cursor = self.input.chars().count();
                }
                prompt_action::none
            }
            KeyCode::Up => {
                let matches = self.completions();
                if !matches.is_empty() {
                    let prev_idx = match self.completion_idx {
                        Some(i) if i > 0 => i - 1,
                        _ => matches.len() - 1,
                    };
                    self.completion_idx = Some(prev_idx);
                    self.input = format!("{} ", matches[prev_idx].name);
                    self.cursor = self.input.chars().count();
                }
                prompt_action::none
            }
            KeyCode::Down => {
                let matches = self.completions();
                if !matches.is_empty() {
                    let next_idx = match self.completion_idx {
                        Some(i) if i + 1 < matches.len() => i + 1,
                        _ => 0,
                    };
                    self.completion_idx = Some(next_idx);
                    self.input = format!("{} ", matches[next_idx].name);
                    self.cursor = self.input.chars().count();
                }
                prompt_action::none
            }
            KeyCode::Backspace => {
                self.completion_idx = None;
                if ctrl {
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
                    prompt_action::cancel
                }
            }
            KeyCode::Delete => {
                self.completion_idx = None;
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
                self.completion_idx = None;
                if ctrl {
                    match ch {
                        'w' => {
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
