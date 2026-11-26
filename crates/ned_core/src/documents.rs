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

/// Unique identifier for an open document buffer.
pub type document_id = usize;

/// In-memory representation of an open text file or scratch buffer.
pub struct document {
    id: document_id,
    text: rope,
    selection: selection,
    path: Option<PathBuf>,
    line_ending: line_ending,
    modified: bool,
    saved_revision: usize,
    history: history,
    highlighter: syntax_highlighter,
    language: Option<language_config>,
    diff_hunks: Vec<hunk>,
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
        let path_buf = path.as_ref().to_path_buf();
        let content = fs::read_to_string(&path_buf)?;
        let buffer = rope::from_str(&content);
        let line_end = detect_line_ending(&buffer);

        let grammar = lang_cfg.as_ref().and_then(|cfg| load_grammar(&cfg.name));
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
        };

        doc.reload_diff();
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
        self.reload_diff();
        Ok(())
    }

    /// Applies an atomic changeset to the document buffer and updates history.
    pub fn apply(&mut self, changes: change_set, new_selection: Option<selection>) {
        if changes.is_empty() {
            if let Some(sel) = new_selection {
                self.selection = sel;
            }
            return;
        }

        let selection_before = self.selection.clone();
        let selection_after = new_selection.unwrap_or_else(|| changes.map_selection(self.selection.clone()));

        self.highlighter.invalidate_from(0);
        self.history.commit(
            changes.clone(),
            &self.text,
            selection_before,
            selection_after.clone(),
        );

        let _ = changes.apply(&mut self.text);
        self.selection = selection_after;
        self.modified = self.history.current_revision() != self.saved_revision;
    }

    /// Undoes the last committed edit.
    pub fn undo(&mut self) -> bool {
        if let Some(restored_sel) = self.history.undo(&mut self.text) {
            self.selection = restored_sel;
            self.highlighter.invalidate_from(0);
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
            self.modified = self.history.current_revision() != self.saved_revision;
            true
        } else {
            false
        }
    }

    /// Reloads the git diff hunks against the repository base version.
    pub fn reload_diff(&mut self) {
        let path = match &self.path {
            Some(p) => p,
            None => return,
        };

        let repo_root = match discover_repo(path) {
            Some(r) => r,
            None => return,
        };

        let relative_path = match path.strip_prefix(&repo_root) {
            Ok(rel) => rel,
            Err(_) => return,
        };

        if let Ok(base_content) = fetch_file_base(&repo_root, relative_path) {
            let current_content = self.text.to_string();
            self.diff_hunks = compute_diff(&base_content, &current_content);
        } else {
            self.diff_hunks.clear();
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

    /// Returns the detected line ending format.
    #[inline]
    #[must_use]
    pub fn line_ending(&self) -> line_ending {
        self.line_ending
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
        let grammar = lang_cfg.as_ref().and_then(|cfg| load_grammar(&cfg.name));
        self.highlighter.set_grammar(grammar);
        self.language = lang_cfg;
    }

    /// Returns git diff hunks computed against the repository index.
    #[inline]
    #[must_use]
    pub fn diff_hunks(&self) -> &[hunk] {
        &self.diff_hunks
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
