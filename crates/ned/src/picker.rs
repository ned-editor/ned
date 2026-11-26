//! Interactive file and buffer picker popup with fuzzy filtering and live preview pane.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use ned_tui::input::events::{KeyCode, KeyEvent, KeyModifier};

/// Minimum terminal area width required to display the split preview pane.
pub const min_area_width_for_preview: u16 = 72;

/// Maximum file size eligible for live text preview (10MB).
pub const max_file_size_for_preview: u64 = 10 * 1024 * 1024;

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

/// Content to display in the live preview pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum preview_content {
    /// Plain text lines from the target file.
    text(Vec<String>),
    /// Binary file not suitable for text preview.
    binary,
    /// Directory listing of entries (entry_name, is_dir).
    directory(Vec<(String, bool)>),
    /// File size exceeds maximum preview limit.
    too_large,
    /// File or directory was not found.
    not_found,
}

impl preview_content {
    /// Returns a placeholder message for non-text preview variants.
    #[must_use]
    pub fn placeholder(&self) -> Option<&'static str> {
        match self {
            Self::text(_) => None,
            Self::binary => Some("<Binary file>"),
            Self::directory(_) => None,
            Self::too_large => Some("<File too large to preview>"),
            Self::not_found => Some("<File not found>"),
        }
    }
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
        collect_files(root_dir, root_dir, &mut ignore_stack, &mut items, 0);
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
        self.ensure_preview_loaded();
    }

    /// Resolves full path and loads preview content for `item`.
    fn load_preview_for_item(&self, item: &str) -> preview_content {
        let full_path = if let Some(root) = &self.root_dir {
            root.join(item)
        } else {
            PathBuf::from(item)
        };
        load_preview_for_path(&full_path)
    }
}

/// Loads preview content for the given file or directory path.
fn load_preview_for_path(path: &Path) -> preview_content {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return preview_content::not_found,
    };

    if metadata.is_dir() {
        let mut entries = Vec::new();
        if let Ok(dir_entries) = fs::read_dir(path) {
            for entry in dir_entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                entries.push((name, is_dir));
            }
        }
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        return preview_content::directory(entries);
    }

    if metadata.len() > max_file_size_for_preview {
        return preview_content::too_large;
    }

    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return preview_content::not_found,
    };

    let mut reader = std::io::BufReader::new(file);
    let mut initial_buf = [0u8; 1024];
    let n = match reader.read(&mut initial_buf) {
        Ok(n) => n,
        Err(_) => return preview_content::not_found,
    };

    // Look for null byte in the initial read chunk to identify binary files
    if initial_buf[..n].contains(&0) {
        return preview_content::binary;
    }

    let full_content = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return preview_content::binary,
    };

    let lines: Vec<String> = full_content.lines().take(1000).map(String::from).collect();
    preview_content::text(lines)
}

/// Recursively traverses directory to collect candidate file paths relative to `root`,
/// strictly respecting active `.gitignore` rules via `ignore_stack`.
fn collect_files(
    root: &Path,
    current: &Path,
    ignore_stack: &mut ned_git::git_ignore_stack,
    out: &mut Vec<String>,
    depth: usize,
) {
    if depth > 12 {
        return;
    }

    let pushed = ignore_stack.push_dir(current);

    if let Ok(entries) = fs::read_dir(current) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Always ignore internal git tracking metadata
            if name_str == ".git" {
                continue;
            }

            let path = entry.path();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

            if ignore_stack.is_ignored(&path, is_dir) {
                continue;
            }

            if is_dir {
                collect_files(root, &path, ignore_stack, out, depth + 1);
            } else if path.is_file() {
                if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }

    if pushed {
        ignore_stack.pop();
    }
}
