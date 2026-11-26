//! Interactive file and buffer picker popup with fuzzy filtering.

use std::fs;
use std::path::Path;

use ned_tui::input::events::{KeyCode, KeyEvent, KeyModifier};

/// Result of processing a key event in picker mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum picker_action {
    /// No picker state change.
    none,
    /// Picker was cancelled (e.g. Esc pressed).
    cancel,
    /// An item was selected and submitted.
    select(String),
}

/// State of an active modal picker overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct picker_state {
    /// Title displayed in the picker window header.
    pub title: String,
    /// User query string.
    pub query: String,
    /// Complete collection of candidate items.
    pub items: Vec<String>,
    /// Indices into `items` matching the current `query`.
    pub filtered: Vec<usize>,
    /// Index in `filtered` currently selected.
    pub selected: usize,
}

impl picker_state {
    /// Creates a new file picker scanning files in `root_dir`.
    #[must_use]
    pub fn new_file_picker(root_dir: &Path) -> Self {
        let mut items = Vec::new();
        collect_files(root_dir, root_dir, &mut items, 0);
        items.sort();

        let filtered = (0..items.len()).collect();
        Self {
            title: "File Picker".to_string(),
            query: String::new(),
            items,
            filtered,
            selected: 0,
        }
    }

    /// Creates a new buffer picker listing open document names.
    #[must_use]
    pub fn new_buffer_picker(buffers: Vec<String>) -> Self {
        let filtered = (0..buffers.len()).collect();
        Self {
            title: "Buffer Picker".to_string(),
            query: String::new(),
            items: buffers,
            filtered,
            selected: 0,
        }
    }

    /// Handles a raw keyboard event while the picker is active.
    pub fn handle_key(&mut self, ev: &KeyEvent) -> picker_action {
        let ctrl = ev.modifiers.contains(&KeyModifier::Control);

        match ev.keycode {
            KeyCode::Esc => picker_action::cancel,
            KeyCode::Enter => {
                if let Some(&idx) = self.filtered.get(self.selected) {
                    if let Some(item) = self.items.get(idx) {
                        return picker_action::select(item.clone());
                    }
                }
                picker_action::cancel
            }
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                picker_action::none
            }
            KeyCode::Down => {
                if !self.filtered.is_empty() && self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                }
                picker_action::none
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.refilter();
                picker_action::none
            }
            KeyCode::Char(ch) => {
                if ctrl {
                    match ch {
                        'p' | 'k' => {
                            if self.selected > 0 {
                                self.selected -= 1;
                            }
                            picker_action::none
                        }
                        'n' | 'j' => {
                            if !self.filtered.is_empty() && self.selected + 1 < self.filtered.len() {
                                self.selected += 1;
                            }
                            picker_action::none
                        }
                        'c' => picker_action::cancel,
                        'u' => {
                            self.query.clear();
                            self.refilter();
                            picker_action::none
                        }
                        _ => picker_action::none,
                    }
                } else {
                    self.query.push(ch);
                    self.refilter();
                    picker_action::none
                }
            }
            _ => picker_action::none,
        }
    }

    /// Re-evaluates matches against `self.query`.
    fn refilter(&mut self) {
        let q = self.query.to_lowercase();
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if q.is_empty() {
                    return true;
                }
                item.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect();

        self.selected = 0;
    }
}

/// Recursively traverses directory to collect candidate file paths relative to `root`.
fn collect_files(root: &Path, current: &Path, out: &mut Vec<String>, depth: usize) {
    if depth > 10 {
        return;
    }

    let Ok(entries) = fs::read_dir(current) else {
        return;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip ignored directories
        if name_str.starts_with('.')
            || name_str == "target"
            || name_str == "node_modules"
            || name_str == "dist"
        {
            continue;
        }

        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out, depth + 1);
        } else if path.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.to_string_lossy().to_string());
            }
        }
    }
}
