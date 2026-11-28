//! Document buffer lifecycle, file I/O, syntax highlighting, diffs, and edits.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ned_config::language_config;
use ned_git::{compute_diff, discover_repo, fetch_file_base, hunk};
use ned_rope::rope;
use ned_text::{change_set, detect_line_ending, line_ending, selection};
use ned_tree::{highlight_span, syntax_highlighter};

use crate::histories::history;
use crate::runtime_loader::load_grammar;

/// Position in a text document (zero-based line and character).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct diagnostic_position {
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based character offset.
    pub character: u32,
}

/// Range within a text document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct diagnostic_range {
    /// Start position.
    pub start: diagnostic_position,
    /// End position.
    pub end: diagnostic_position,
}

/// Diagnostic severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum diagnostic_severity {
    /// Error severity.
    error,
    /// Warning severity.
    warning,
    /// Information severity.
    information,
    /// Hint severity.
    hint,
}

/// Lightweight diagnostic representation for editor document annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct diagnostic {
    /// Range of text affected by the diagnostic.
    pub range: diagnostic_range,
    /// Severity level.
    pub severity: Option<diagnostic_severity>,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Unique identifier for an open document buffer.
pub type document_id = usize;

/// In-memory representation of an open text file or scratch buffer.
pub struct document {
    /// Unique identifier for this document buffer.
    id: document_id,
    /// Underlying text rope buffer.
    text: rope,
    /// Active multi-selection.
    selection: selection,
    /// Optional file path this document is backed by.
    path: Option<PathBuf>,
    /// Detected line ending format.
    line_ending: line_ending,
    /// Whether the document has unpersisted changes.
    modified: bool,
    /// History revision at last save.
    saved_revision: usize,
    /// Edit history for undo/redo.
    history: history,
    /// Syntax highlighter for this document.
    highlighter: syntax_highlighter,
    /// Associated language configuration, if any.
    language: Option<language_config>,
    /// Computed git diff hunks.
    diff_hunks: Vec<hunk>,
    /// True if git diff was skipped due to size limits.
    diff_too_large: bool,
    /// Active compiler diagnostics.
    diagnostics: Vec<diagnostic>,
    /// Monotonically increasing document version.
    version: i32,
}

impl document {
    /// Creates a new empty scratch document.
    #[must_use]
    pub fn new_empty(id: document_id) -> Self {
        Self {
            id,
            text: rope::new(),
            selection: selection::point(0),
            path: None,
            line_ending: line_ending::lf,
            modified: false,
            saved_revision: 0,
            history: history::new(),
            highlighter: syntax_highlighter::new(None),
            language: None,
            diff_hunks: Vec::new(),
            diff_too_large: false,
            diagnostics: Vec::new(),
            version: 1,
        }
    }

    /// Creates a new document from an existing text string.
    #[must_use]
    pub fn from_string(id: document_id, text: &str) -> Self {
        let r = rope::from_string(text);
        let le = ned_text::detect_line_ending(&r);
        Self {
            id,
            text: r,
            selection: selection::point(0),
            path: None,
            line_ending: le,
            modified: false,
            saved_revision: 0,
            history: history::new(),
            highlighter: syntax_highlighter::new(None),
            language: None,
            diff_hunks: Vec::new(),
            diff_too_large: false,
            diagnostics: Vec::new(),
            version: 1,
        }
    }

    /// Opens an existing file from disk into a new document buffer.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if reading the file from disk fails.
    pub fn open(
        id: document_id,
        path: impl AsRef<Path>,
        lang_cfg: Option<language_config>,
    ) -> io::Result<Self> {
        let path_ref = path.as_ref();
        let path_buf = if path_ref.is_absolute() {
            path_ref.to_path_buf()
        } else if let Ok(cwd) = std::env::current_dir() {
            cwd.join(path_ref)
        } else {
            path_ref.to_path_buf()
        };
        let content = fs::read_to_string(&path_buf)?;
        let buffer = rope::from_string(&content);
        let line_end = detect_line_ending(&buffer);

        let grammar = lang_cfg
            .as_ref()
            .and_then(|cfg| load_grammar(cfg.grammar.as_deref().unwrap_or(&cfg.name)));
        let highlighter = syntax_highlighter::new(grammar);

        let mut doc = Self {
            id,
            text: buffer,
            selection: selection::point(0),
            path: Some(path_buf),
            line_ending: line_end,
            modified: false,
            saved_revision: 0,
            history: history::new(),
            highlighter,
            language: lang_cfg,
            diff_hunks: Vec::new(),
            diff_too_large: false,
            diagnostics: Vec::new(),
            version: 1,
        };

        let _ = doc.reload_diff();
        Ok(doc)
    }

    /// Saves the document back to its existing file path on disk.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if the document has no associated file path or if disk writing fails.
    pub fn save(&mut self) -> io::Result<()> {
        let path = match &self.path {
            Some(p) => p.clone(),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "cannot save document without path",
                ))
            }
        };
        self.save_to(&path)
    }

    /// Saves the document to a specified file path on disk.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if writing to disk fails.
    pub fn save_to(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let path_buf = path.as_ref().to_path_buf();
        let content = self.text.to_string();
        fs::write(&path_buf, content)?;

        self.path = Some(path_buf);
        self.saved_revision = self.history.current_revision();
        self.modified = false;
        let _ = self.reload_diff();
        Ok(())
    }

    /// Applies an atomic changeset to the document buffer and updates history.
    pub fn apply(&mut self, changes: &change_set, new_selection: Option<selection>) {
        if changes.is_empty() {
            if let Some(sel) = new_selection {
                self.selection = sel;
            }
            return;
        }

        let selection_before = self.selection.clone();
        let selection_after =
            new_selection.unwrap_or_else(|| changes.map_selection(self.selection.clone()));

        self.highlighter.invalidate_from(0);
        self.history.commit(
            changes.clone(),
            &self.text,
            selection_before,
            selection_after.clone(),
        );

        let _ = changes.apply(&mut self.text);
        self.selection = selection_after;
        self.version += 1;
        self.modified = self.history.current_revision() != self.saved_revision;
    }

    /// Undoes the last committed edit.
    pub fn undo(&mut self) -> bool {
        if let Some(restored_sel) = self.history.undo(&mut self.text) {
            self.selection = restored_sel;
            self.highlighter.invalidate_from(0);
            self.version += 1;
            self.modified = self.history.current_revision() != self.saved_revision;
            true
        } else {
            false
        }
    }

    /// Redoes the previously undone edit.
    pub fn redo(&mut self) -> bool {
        if let Some(restored_sel) = self.history.redo(&mut self.text) {
            self.selection = restored_sel;
            self.highlighter.invalidate_from(0);
            self.version += 1;
            self.modified = self.history.current_revision() != self.saved_revision;
            true
        } else {
            false
        }
    }

    /// Reloads the git diff hunks against the repository base version.
    ///
    /// # Errors
    /// Returns `Err("diff too large")` if either base content or document text exceeds 1 MiB or 20,000 lines.
    pub fn reload_diff(&mut self) -> Result<(), &'static str> {
        let Some(path) = &self.path else {
            self.diff_too_large = false;
            self.diff_hunks.clear();
            return Ok(());
        };

        let Some(repo_root) = discover_repo(path) else {
            self.diff_too_large = false;
            self.diff_hunks.clear();
            return Ok(());
        };

        let Ok(relative_path) = path.strip_prefix(&repo_root) else {
            self.diff_too_large = false;
            self.diff_hunks.clear();
            return Ok(());
        };

        if self.text.byte_len() > 1024 * 1024 || self.text.line_count() > 20_000 {
            self.diff_too_large = true;
            self.diff_hunks.clear();
            return Err("diff too large");
        }

        if let Ok(base_content) = fetch_file_base(&repo_root, relative_path) {
            if base_content.len() > 1024 * 1024 || base_content.lines().count() > 20_000 {
                self.diff_too_large = true;
                self.diff_hunks.clear();
                return Err("diff too large");
            }
            self.diff_too_large = false;
            let current_content = self.text.to_string();
            self.diff_hunks = compute_diff(&base_content, &current_content);
            Ok(())
        } else {
            self.diff_too_large = false;
            self.diff_hunks.clear();
            Ok(())
        }
    }

    /// Returns the unique document identifier.
    #[inline]
    #[must_use]
    pub fn id(&self) -> document_id {
        self.id
    }

    /// Returns a reference to the underlying text rope buffer.
    #[inline]
    #[must_use]
    pub fn text(&self) -> &rope {
        &self.text
    }

    /// Returns a reference to the active multi-selection.
    #[inline]
    #[must_use]
    pub fn selection(&self) -> &selection {
        &self.selection
    }

    /// Sets the active multi-selection.
    pub fn set_selection(&mut self, sel: selection) {
        self.selection = sel;
    }

    /// Returns the optional file path of this document.
    #[inline]
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Sets the file path associated with this document.
    pub fn set_path(&mut self, path: impl AsRef<Path>) {
        self.path = Some(path.as_ref().to_path_buf());
    }

    /// Returns the detected line ending format.
    #[inline]
    #[must_use]
    pub fn line_ending(&self) -> line_ending {
        self.line_ending
    }

    /// Sets the document default line ending format.
    #[inline]
    pub fn set_line_ending(&mut self, le: line_ending) {
        self.line_ending = le;
    }

    /// Returns whether this document has unpersisted changes.
    #[inline]
    #[must_use]
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Returns the associated language configuration, if configured.
    #[inline]
    #[must_use]
    pub fn language(&self) -> Option<&language_config> {
        self.language.as_ref()
    }

    /// Updates language configuration and re-initializes syntax highlighter.
    pub fn set_language(&mut self, lang_cfg: Option<language_config>) {
        let grammar = lang_cfg
            .as_ref()
            .and_then(|cfg| load_grammar(cfg.grammar.as_deref().unwrap_or(&cfg.name)));
        self.highlighter.set_grammar(grammar);
        self.language = lang_cfg;
    }

    /// Returns git diff hunks computed against the repository index.
    #[inline]
    #[must_use]
    pub fn diff_hunks(&self) -> &[hunk] {
        &self.diff_hunks
    }

    /// Returns true if git diff computation was skipped because file exceeded resource limits.
    #[inline]
    #[must_use]
    pub fn is_diff_too_large(&self) -> bool {
        self.diff_too_large
    }

    /// Returns the monotonically increasing document version.
    #[inline]
    #[must_use]
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Returns the active compiler diagnostics for this document.
    #[inline]
    #[must_use]
    pub fn diagnostics(&self) -> &[diagnostic] {
        &self.diagnostics
    }

    /// Sets the active compiler diagnostics for this document.
    pub fn set_diagnostics(&mut self, diags: Vec<diagnostic>) {
        self.diagnostics = diags;
    }

    /// Returns a mutable reference to the syntax highlighter.
    #[inline]
    pub fn highlighter_mut(&mut self) -> &mut syntax_highlighter {
        &mut self.highlighter
    }

    /// Returns an immutable reference to the syntax highlighter.
    #[inline]
    #[must_use]
    pub fn highlighter(&self) -> &syntax_highlighter {
        &self.highlighter
    }

    /// Highlights viewport lines and returns line-indexed highlight spans.
    #[must_use]
    pub fn highlight_viewport(
        &mut self,
        start_line: usize,
        end_line: usize,
    ) -> Vec<(usize, Vec<highlight_span>)> {
        self.highlighter
            .highlight_viewport(&self.text, start_line, end_line)
    }
}
