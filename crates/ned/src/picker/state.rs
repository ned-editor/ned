//! State representation and interactive key event dispatch for modal pickers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ned_tui::input::events::{KeyCode, KeyEvent, KeyModifier};

use crate::picker::filesystem;
use crate::picker::filter;
use crate::picker::preview::{self, preview_content};

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
    /// Root directory for resolving relative file paths.
    pub root_dir: Option<PathBuf>,
    /// Whether the preview pane is currently enabled.
    pub show_preview: bool,
    /// Cached preview contents keyed by item path string.
    pub preview_cache: HashMap<String, preview_content>,
}

impl picker_state {
    /// Creates a new file picker scanning files in `root_dir`.
    #[must_use]
    pub fn new_file_picker(root_dir: &Path) -> Self {
        let mut items = Vec::new();
        let mut ignore_stack = ned_git::git_ignore_stack::new();
        filesystem::collect_files(root_dir, root_dir, &mut ignore_stack, &mut items, 0);
        items.sort();

        let filtered = (0..items.len()).collect();
        let mut state = Self {
            title: "File Picker".to_string(),
            query: String::new(),
            items,
            filtered,
            selected: 0,
            root_dir: Some(root_dir.to_path_buf()),
            show_preview: true,
            preview_cache: HashMap::new(),
        };
        state.ensure_preview_loaded();
        state
    }

    /// Creates a new buffer picker listing open document names.
    #[must_use]
    pub fn new_buffer_picker(buffers: Vec<String>) -> Self {
        let filtered = (0..buffers.len()).collect();
        let root_dir = std::env::current_dir().ok();
        let mut state = Self {
            title: "Buffer Picker".to_string(),
            query: String::new(),
            items: buffers,
            filtered,
            selected: 0,
            root_dir,
            show_preview: true,
            preview_cache: HashMap::new(),
        };
        state.ensure_preview_loaded();
        state
    }

    /// Creates a new diagnostics picker listing formatted LSP diagnostics.
    #[must_use]
    pub fn new_diagnostics_picker(items: Vec<String>) -> Self {
        let filtered = (0..items.len()).collect();
        Self {
            title: "Diagnostics".to_string(),
            query: String::new(),
            items,
            filtered,
            selected: 0,
            root_dir: None,
            show_preview: false,
            preview_cache: HashMap::new(),
        }
    }

    /// Returns the currently selected item string, if any.
    #[must_use]
    pub fn selected_item(&self) -> Option<&str> {
        self.filtered
            .get(self.selected)
            .and_then(|&idx| self.items.get(idx))
            .map(String::as_str)
    }

    /// Returns the preview content for the currently selected item.
    #[must_use]
    pub fn get_preview(&self) -> Option<preview_content> {
        if !self.show_preview {
            return None;
        }
        let item = self.selected_item()?;
        if let Some(cached) = self.preview_cache.get(item) {
            return Some(cached.clone());
        }
        Some(self.load_preview_for_item(item))
    }

    /// Toggles preview pane visibility on/off.
    pub fn toggle_preview(&mut self) {
        self.show_preview = !self.show_preview;
        if self.show_preview {
            self.ensure_preview_loaded();
        }
    }

    /// Moves selection up by one item.
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.ensure_preview_loaded();
        }
    }

    /// Moves selection down by one item.
    pub fn move_down(&mut self) {
        if !self.filtered.is_empty() && self.selected + 1 < self.filtered.len() {
            self.selected += 1;
            self.ensure_preview_loaded();
        }
    }

    /// Scrolls selection up by a full page (10 items).
    pub fn page_up(&mut self) {
        if self.selected > 0 {
            self.selected = self.selected.saturating_sub(10);
            self.ensure_preview_loaded();
        }
    }

    /// Scrolls selection down by a full page (10 items).
    pub fn page_down(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 10).min(self.filtered.len().saturating_sub(1));
            self.ensure_preview_loaded();
        }
    }

    /// Ensures the preview for the currently selected item is cached.
    pub fn ensure_preview_loaded(&mut self) {
        if !self.show_preview {
            return;
        }
        if let Some(item) = self.selected_item() {
            let item_str = item.to_string();
            if !self.preview_cache.contains_key(&item_str) {
                let content = self.load_preview_for_item(&item_str);
                self.preview_cache.insert(item_str, content);
            }
        }
    }

    /// Handles a raw keyboard event while the picker is active.
    pub fn handle_key(&mut self, ev: &KeyEvent) -> picker_action {
        let ctrl = ev.modifiers.contains(&KeyModifier::Control);
        let shift = ev.modifiers.contains(&KeyModifier::Shift);

        match ev.keycode {
            KeyCode::Esc => picker_action::cancel,
            KeyCode::Enter => {
                if let Some(item) = self.selected_item() {
                    return picker_action::select(item.to_string());
                }
                picker_action::cancel
            }
            KeyCode::Up => {
                self.move_up();
                picker_action::none
            }
            KeyCode::Down => {
                self.move_down();
                picker_action::none
            }
            KeyCode::Tab => {
                if shift {
                    self.move_up();
                } else {
                    self.move_down();
                }
                picker_action::none
            }
            KeyCode::BackTab => {
                self.move_up();
                picker_action::none
            }
            KeyCode::PageUp => {
                self.page_up();
                picker_action::none
            }
            KeyCode::PageDown => {
                self.page_down();
                picker_action::none
            }
            KeyCode::Home => {
                self.selected = 0;
                self.ensure_preview_loaded();
                picker_action::none
            }
            KeyCode::End => {
                self.selected = self.filtered.len().saturating_sub(1);
                self.ensure_preview_loaded();
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
                            self.move_up();
                            picker_action::none
                        }
                        'n' | 'j' => {
                            self.move_down();
                            picker_action::none
                        }
                        'u' => {
                            self.page_up();
                            picker_action::none
                        }
                        'd' => {
                            self.page_down();
                            picker_action::none
                        }
                        't' => {
                            self.toggle_preview();
                            picker_action::none
                        }
                        'c' => picker_action::cancel,
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
        self.filtered = filter::filter_candidates(&self.query, &self.items);
        self.selected = 0;
        self.ensure_preview_loaded();
    }

    /// Resolves full path and loads preview content for `item`.
    fn load_preview_for_item(&self, item: &str) -> preview_content {
        preview::load_preview_for_item(self.root_dir.as_deref(), item)
    }
}
