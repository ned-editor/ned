//! Editor state machine, multi-document management, keymap dispatch, and action execution.

use std::collections::HashMap;
use std::path::Path;

use ::ned_config::{action, key_event, key_trie_node, mode, ned_config};
use ned_text::{
    apply_motion, direction, extend_line, move_first_non_whitespace, move_horizontally,
    move_line_end, move_line_start, move_long_word_backward, move_long_word_end,
    move_long_word_forward, move_vertically, move_word_backward, move_word_end, move_word_forward,
    movement, range, selection,
};

use crate::documents::{document, document_id};
use crate::registers::register_storage;
use crate::views::{view, view_id};

pub mod command_exec;
pub mod edit_ops;
pub mod git_ops;
pub mod indent_comment;
pub mod mode_dispatch;
pub mod view_ops;

/// Central editor runtime coordinating documents, views, registers, and modal input.
pub struct editor {
    /// Open documents keyed by unique document identifier.
    pub documents: HashMap<document_id, document>,
    /// Identifier of the currently focused document.
    pub active_doc_id: document_id,
    /// Open views keyed by view identifier.
    pub views: HashMap<view_id, view>,
    /// Identifier of the currently active view.
    pub active_view_id: view_id,
    /// Active editor mode (normal, insert, select).
    pub mode: mode,
    /// User and default configuration settings.
    pub config: ned_config,
    /// Clipboard and register storage.
    pub registers: register_storage,
    /// Pending numerical count prefix (e.g. `3w`).
    pub count: Option<usize>,
    /// Active register prefix key if explicitly selected (e.g. `"a`).
    pub selected_register: Option<char>,
    /// Pending character replace operation flag (`r`).
    pub pending_replace: bool,
    /// Pending textobject selection mode (`ma` or `mi`).
    pub pending_textobject: Option<ned_text::textobject_mode>,
    /// Pending surround operation (`ms`, `md`, `mr`).
    pub pending_surround: Option<crate::surround::surround_op>,
    /// History of selections before syntax expansions for shrink navigation (`Alt-i`).
    pub syntax_history: Vec<(usize, ned_text::selection)>,
    /// Pending key prefix tree for multi-keystroke combinations (e.g. `g`, `]`, `[`).
    pub key_prefix: Option<key_trie_node>,
    /// Ephemeral status line message.
    pub status_message: Option<String>,
    /// Floating hover documentation popup text.
    pub hover_popup: Option<String>,
    /// Active search pattern query string.
    pub search_query: Option<String>,
    /// Termination flag signaling editor exit.
    pub should_quit: bool,
    /// Optional exit status code when quitting via :cquit.
    pub exit_code: Option<i32>,
    pub(crate) next_doc_id: usize,
    pub(crate) next_view_id: usize,
}

impl editor {
    /// Creates a new editor instance with an initial empty scratch document.
    #[must_use]
    pub fn new(config: ned_config, width: u16, height: u16) -> Self {
        let initial_doc_id = 1;
        let initial_view_id = 1;

        let mut docs = HashMap::new();
        docs.insert(initial_doc_id, document::new_empty(initial_doc_id));

        let mut views = HashMap::new();
        views.insert(
            initial_view_id,
            view::new(initial_view_id, initial_doc_id, width, height),
        );

        let status_message = config
            .warnings
            .first()
            .map(|w| format!("Config warning: {w}"));

        Self {
            documents: docs,
            active_doc_id: initial_doc_id,
            views,
            active_view_id: initial_view_id,
            mode: mode::normal,
            config,
            registers: register_storage::new(),
            count: None,
            selected_register: None,
            pending_replace: false,
            pending_textobject: None,
            pending_surround: None,
            syntax_history: Vec::new(),
            key_prefix: None,
            status_message,
            hover_popup: None,
            search_query: None,
            should_quit: false,
            exit_code: None,
            next_doc_id: 2,
            next_view_id: 2,
        }
    }

    /// Opens a file from disk into a new document buffer and focuses its view.
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if reading the file from disk fails.
    pub fn open_file(&mut self, path: impl AsRef<Path>) -> std::io::Result<document_id> {
        let path_ref = path.as_ref();
        let abs_path = if path_ref.is_absolute() {
            path_ref.to_path_buf()
        } else if let Ok(cwd) = std::env::current_dir() {
            cwd.join(path_ref)
        } else {
            path_ref.to_path_buf()
        };
        let lang_cfg = self.config.languages.detect_by_path(&abs_path).cloned();

        let doc_id = self.next_doc_id;
        self.next_doc_id += 1;

        let doc = match document::open(doc_id, &abs_path, lang_cfg.clone()) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut d = document::new_empty(doc_id);
                d.set_path(&abs_path);
                d.set_language(lang_cfg);
                d
            }
            Err(e) => return Err(e),
        };
        self.documents.insert(doc_id, doc);
        self.active_doc_id = doc_id;

        if let Some(v) = self.views.get_mut(&self.active_view_id) {
            v.doc_id = doc_id;
            v.row_offset = 0;
            v.col_offset = 0;
        }

        self.ensure_active_cursor_visible();
        git_ops::refresh_diff(self, doc_id);
        Ok(doc_id)
    }

    /// Creates a new view displaying `doc_id` with specified dimensions.
    #[inline]
    pub fn create_view(&mut self, doc_id: document_id, width: u16, height: u16) -> view_id {
        view_ops::create_view(self, doc_id, width, height)
    }

    /// Returns a reference to the active document.
    #[inline]
    #[must_use]
    pub fn active_doc(&self) -> &document {
        self.documents
            .get(&self.active_doc_id)
            .expect("active document must exist")
    }

    /// Returns a mutable reference to the active document.
    #[inline]
    pub fn active_doc_mut(&mut self) -> &mut document {
        self.documents
            .get_mut(&self.active_doc_id)
            .expect("active document must exist")
    }

    /// Returns a reference to the active view.
    #[inline]
    #[must_use]
    pub fn active_view(&self) -> &view {
        view_ops::active_view(self)
    }

    /// Returns a mutable reference to the active view.
    #[inline]
    pub fn active_view_mut(&mut self) -> &mut view {
        view_ops::active_view_mut(self)
    }

    /// Ensures the active cursor is within the viewport margins.
    #[inline]
    pub fn ensure_active_cursor_visible(&mut self) {
        view_ops::ensure_active_cursor_visible(self);
    }

    /// Handles a mouse click event at screen coordinates `(col, row)`.
    #[inline]
    pub fn handle_mouse_click(
        &mut self,
        col: u16,
        row: u16,
        gutter_width: u16,
        editor_height: u16,
        extend: bool,
    ) {
        view_ops::handle_mouse_click(self, col, row, gutter_width, editor_height, extend);
    }

    /// Scrolls the active view vertically by `delta` lines matching Helix scroll behavior.
    ///
    /// Preserves primary cursor position unless scrolled past the viewport margins (scrolloff).
    #[inline]
    pub fn scroll_view_by(&mut self, delta: isize) {
        view_ops::scroll_view_by(self, delta);
    }

    /// Processes an incoming keyboard key event.
    ///
    /// Returns `true` if the event was handled and may require redraw.
    #[inline]
    pub fn handle_key(&mut self, ev: key_event) -> bool {
        mode_dispatch::handle_key(self, ev)
    }

    /// Dispatches an editor action.
    pub fn dispatch(&mut self, act: action) {
        match act {
            // --- Motions ---
            action::move_char_left
            | action::move_char_right
            | action::move_line_up
            | action::move_line_down
            | action::move_next_word_start
            | action::move_prev_word_start
            | action::move_next_word_end
            | action::move_next_long_word_start
            | action::move_prev_long_word_start
            | action::move_next_long_word_end
            | action::goto_line_start
            | action::goto_line_end
            | action::goto_first_nonwhitespace
            | action::goto_file_start
            | action::goto_file_end
            | action::goto_line => {
                self.execute_motion(act);
            }

            // --- Line extension ---
            action::extend_line_below => {
                let doc = self.active_doc_mut();
                let sel = apply_motion(doc.text(), doc.selection(), |r| {
                    extend_line(doc.text(), r, 1)
                });
                doc.set_selection(sel);
                self.ensure_active_cursor_visible();
            }
            action::extend_to_line_bounds => {
                let doc = self.active_doc_mut();
                let sel = apply_motion(doc.text(), doc.selection(), |r| {
                    let buffer = doc.text();
                    let cursor_pos = r.cursor(buffer);
                    let line = buffer.byte_to_line(buffer.char_to_byte(cursor_pos));
                    let start_byte = buffer.line_to_byte(line).unwrap_or(0);
                    let start_char = buffer.byte_to_char(start_byte);
                    let end_byte = buffer
                        .line_to_byte(line + 1)
                        .unwrap_or_else(|| buffer.byte_len());
                    let end_char = buffer.byte_to_char(end_byte);
                    range::new(start_char, end_char)
                });
                doc.set_selection(sel);
                self.ensure_active_cursor_visible();
            }

            // --- Mode switching ---
            action::insert_mode => {
                self.mode = mode::insert;
            }
            action::append_mode => {
                let doc = self.active_doc_mut();
                let sel = apply_motion(doc.text(), doc.selection(), |r| {
                    move_horizontally(doc.text(), r, direction::forward, 1, movement::move_)
                });
                doc.set_selection(sel);
                self.mode = mode::insert;
                self.ensure_active_cursor_visible();
            }
            action::insert_at_line_start => {
                let doc = self.active_doc_mut();
                let sel = apply_motion(doc.text(), doc.selection(), |r| {
                    move_first_non_whitespace(doc.text(), r, movement::move_)
                });
                doc.set_selection(sel);
                self.mode = mode::insert;
                self.ensure_active_cursor_visible();
            }
            action::insert_at_line_end => {
                let doc = self.active_doc_mut();
                let sel = apply_motion(doc.text(), doc.selection(), |r| {
                    move_line_end(doc.text(), r, movement::move_)
                });
                doc.set_selection(sel);
                self.mode = mode::insert;
                self.ensure_active_cursor_visible();
            }
            action::open_below => {
                {
                    let doc = self.active_doc_mut();
                    let sel = apply_motion(doc.text(), doc.selection(), |r| {
                        move_line_end(doc.text(), r, movement::move_)
                    });
                    doc.set_selection(sel);
                }
                self.insert_char('\n');
                self.mode = mode::insert;
            }
            action::open_above => {
                {
                    let doc = self.active_doc_mut();
                    let sel = apply_motion(doc.text(), doc.selection(), |r| {
                        move_line_start(doc.text(), r, movement::move_)
                    });
                    doc.set_selection(sel);
                }
                self.insert_char('\n');
                {
                    let doc = self.active_doc_mut();
                    let sel_up = apply_motion(doc.text(), doc.selection(), |r| {
                        move_vertically(doc.text(), r, direction::backward, 1, movement::move_)
                    });
                    doc.set_selection(sel_up);
                }
                self.mode = mode::insert;
            }
            action::select_mode => {
                self.mode = mode::select;
            }
            action::normal_mode => {
                self.mode = mode::normal;
            }
            action::command_mode => {
                self.status_message = Some(":".to_string());
            }

            // --- Editing & Deletion ---
            action::delete_selection => {
                self.delete_selection(true);
            }
            action::delete_selection_noyank => {
                self.delete_selection(false);
            }
            action::change_selection => {
                self.change_selection(true);
            }
            action::change_selection_noyank => {
                self.change_selection(false);
            }
            action::replace => {
                self.pending_replace = true;
                self.status_message = Some("replace: ".to_string());
            }
            action::replace_with_yanked => {
                self.replace_with_yanked();
            }
            action::yank => {
                self.yank_selection();
            }
            action::paste_after => {
                self.paste_register(true);
            }
            action::paste_before => {
                self.paste_register(false);
            }
            action::undo => {
                let _changed = self.active_doc_mut().undo();
                self.ensure_active_cursor_visible();
            }
            action::redo => {
                let _changed = self.active_doc_mut().redo();
                self.ensure_active_cursor_visible();
            }
            action::switch_case | action::switch_to_lowercase | action::switch_to_uppercase => {
                self.transform_selection_case(act);
            }
            action::indent => {
                self.indent_lines(false);
            }
            action::unindent => {
                self.indent_lines(true);
            }
            action::toggle_line_comment => {
                self.toggle_line_comment();
            }
            action::toggle_block_comment => {
                self.toggle_block_comment();
            }

            // --- Selection operations ---
            action::select_all => {
                let doc = self.active_doc_mut();
                doc.set_selection(selection::single(0, doc.text().char_len()));
                self.ensure_active_cursor_visible();
            }
            action::collapse_selection => {
                let doc = self.active_doc_mut();
                let sel = doc.selection().collapse(doc.text());
                doc.set_selection(sel);
                self.ensure_active_cursor_visible();
            }
            action::flip_selections => {
                let doc = self.active_doc_mut();
                doc.set_selection(doc.selection().flip());
                self.ensure_active_cursor_visible();
            }
            action::ensure_selection_forward => {
                let doc = self.active_doc_mut();
                doc.set_selection(doc.selection().ensure_forward());
                self.ensure_active_cursor_visible();
            }
            action::keep_primary_selection => {
                let doc = self.active_doc_mut();
                doc.set_selection(doc.selection().clone().into_single());
                self.ensure_active_cursor_visible();
            }
            action::remove_primary_selection => {
                let doc = self.active_doc_mut();
                doc.set_selection(doc.selection().remove_primary());
                self.ensure_active_cursor_visible();
            }
            action::rotate_selections_forward => {
                let count = self.count.take().unwrap_or(1);
                let doc = self.active_doc_mut();
                let mut sel = doc.selection().clone();
                sel.cycle_primary_forward(count);
                doc.set_selection(sel);
                self.ensure_active_cursor_visible();
            }
            action::rotate_selections_backward => {
                let count = self.count.take().unwrap_or(1);
                let doc = self.active_doc_mut();
                let mut sel = doc.selection().clone();
                sel.cycle_primary_backward(count);
                doc.set_selection(sel);
                self.ensure_active_cursor_visible();
            }
            action::rotate_selection_contents_forward => {
                let count = self.count.take().unwrap_or(1);
                self.rotate_selection_contents(true, count);
            }
            action::rotate_selection_contents_backward => {
                let count = self.count.take().unwrap_or(1);
                self.rotate_selection_contents(false, count);
            }
            action::merge_consecutive_selections => {
                let doc = self.active_doc_mut();
                doc.set_selection(doc.selection().clone().merge_consecutive());
                self.ensure_active_cursor_visible();
            }
            action::trim_selections => {
                let doc = self.active_doc_mut();
                doc.set_selection(doc.selection().trim_whitespace(doc.text()));
                self.ensure_active_cursor_visible();
            }
            action::split_selection_on_newline => {
                let doc = self.active_doc_mut();
                doc.set_selection(doc.selection().split_on_newline(doc.text()));
                self.ensure_active_cursor_visible();
            }
            action::shrink_to_line_bounds => {
                self.shrink_to_line_bounds();
            }
            action::align_selections => {
                self.align_selections();
            }
            action::join_selections => {
                self.join_selections(true);
            }
            action::join_selections_space => {
                self.join_selections(false);
            }
            action::copy_selection_on_next_line => {
                let count = self.count.take().unwrap_or(1);
                self.copy_selection_on_line(true, count);
            }
            action::copy_selection_on_prev_line => {
                let count = self.count.take().unwrap_or(1);
                self.copy_selection_on_line(false, count);
            }
            action::select_regex => {
                self.status_message = Some("prompt:select".to_string());
            }
            action::split_selection => {
                self.status_message = Some("prompt:split".to_string());
            }
            action::keep_matching_selections => {
                self.status_message = Some("prompt:keep".to_string());
            }
            action::remove_matching_selections => {
                self.status_message = Some("prompt:remove".to_string());
            }

            // --- Search ---
            action::search | action::rsearch => {
                self.status_message = Some("/".to_string());
            }
            action::search_next => {
                self.jump_search(true);
            }
            action::search_prev => {
                self.jump_search(false);
            }

            // --- File operations ---
            action::write => {
                let res = self.active_doc_mut().save();
                match res {
                    Ok(()) => {
                        self.status_message = Some("written".to_string());
                        git_ops::refresh_diff(self, self.active_doc_id);
                    }
                    Err(e) => self.status_message = Some(format!("write error: {e}")),
                }
            }
            action::quit => {
                let doc = self.active_doc();
                if doc.is_modified() {
                    self.status_message = Some("unsaved changes (use :q! to force)".to_string());
                } else {
                    self.should_quit = true;
                }
            }
            action::force_quit => {
                self.should_quit = true;
            }

            // --- Insert mode primitives ---
            action::insert_newline => {
                self.insert_newline();
            }
            action::insert_tab => {
                let tab_str = " ".repeat(self.config.editor.tab_width);
                self.insert_string(&tab_str);
            }
            action::delete_char_backward => {
                self.delete_backward();
            }
            action::delete_word_backward => {
                // Delete word backward
            }

            // --- Language & editor actions ---
            action::goto_next_diag
            | action::goto_prev_diag
            | action::hover
            | action::goto_definition
            | action::goto_reference => {}
            action::goto_next_hunk => {
                self.goto_next_hunk();
            }
            action::goto_prev_hunk => {
                self.goto_prev_hunk();
            }
            action::file_picker => {
                self.status_message = Some("picker:file".to_string());
            }
            action::global_search => {
                self.status_message = Some("picker:global_search".to_string());
            }
            action::buffer_picker => {
                self.status_message = Some("picker:buffer".to_string());
            }
            action::diagnostics_picker => {
                self.status_message = Some("picker:diagnostics".to_string());
            }

            // --- View alignment & scrolling ---
            action::align_view_center => {
                view_ops::align_view(self, view_ops::view_align::center);
            }
            action::align_view_top => {
                view_ops::align_view(self, view_ops::view_align::top);
            }
            action::align_view_bottom => {
                view_ops::align_view(self, view_ops::view_align::bottom);
            }
            action::scroll_down => {
                let lines = (self.count.take().unwrap_or(1) as isize) * 5;
                view_ops::scroll_view_by(self, lines);
            }
            action::scroll_up => {
                let lines = (self.count.take().unwrap_or(1) as isize) * 5;
                view_ops::scroll_view_by(self, -lines);
            }

            // --- Window and buffer navigation ---
            action::goto_window_top => {
                let view_id = self.active_view_id;
                let row_offset = self.views.get(&view_id).map_or(0, |v| v.row_offset);
                let doc = self.active_doc_mut();
                let buffer = doc.text();
                let byte = buffer.line_to_byte(row_offset).unwrap_or(0);
                doc.set_selection(selection::point(buffer.byte_to_char(byte)));
                self.ensure_active_cursor_visible();
            }
            action::goto_window_center => {
                let view_id = self.active_view_id;
                let (row_offset, height) = self
                    .views
                    .get(&view_id)
                    .map_or((0, 24), |v| (v.row_offset, v.height));
                let doc = self.active_doc_mut();
                let buffer = doc.text();
                let max_line = buffer.line_count().saturating_sub(1);
                let target_line = (row_offset + (height as usize / 2)).min(max_line);
                let byte = buffer.line_to_byte(target_line).unwrap_or(0);
                doc.set_selection(selection::point(buffer.byte_to_char(byte)));
                self.ensure_active_cursor_visible();
            }
            action::goto_window_bottom => {
                let view_id = self.active_view_id;
                let (row_offset, height) = self
                    .views
                    .get(&view_id)
                    .map_or((0, 24), |v| (v.row_offset, v.height));
                let doc = self.active_doc_mut();
                let buffer = doc.text();
                let max_line = buffer.line_count().saturating_sub(1);
                let target_line = (row_offset + (height as usize).saturating_sub(1)).min(max_line);
                let byte = buffer.line_to_byte(target_line).unwrap_or(0);
                doc.set_selection(selection::point(buffer.byte_to_char(byte)));
                self.ensure_active_cursor_visible();
            }
            action::goto_next_buffer => {
                let doc_ids: Vec<document_id> = self.documents.keys().copied().collect();
                if doc_ids.len() > 1 {
                    if let Some(pos) = doc_ids.iter().position(|&id| id == self.active_doc_id) {
                        let next_pos = (pos + 1) % doc_ids.len();
                        let next_id = doc_ids[next_pos];
                        self.active_doc_id = next_id;
                        if let Some(v) = self.views.get_mut(&self.active_view_id) {
                            v.doc_id = next_id;
                        }
                        self.ensure_active_cursor_visible();
                    }
                }
            }
            action::goto_previous_buffer => {
                let doc_ids: Vec<document_id> = self.documents.keys().copied().collect();
                if doc_ids.len() > 1 {
                    if let Some(pos) = doc_ids.iter().position(|&id| id == self.active_doc_id) {
                        let prev_pos = if pos == 0 { doc_ids.len() - 1 } else { pos - 1 };
                        let prev_id = doc_ids[prev_pos];
                        self.active_doc_id = prev_id;
                        if let Some(v) = self.views.get_mut(&self.active_view_id) {
                            v.doc_id = prev_id;
                        }
                        self.ensure_active_cursor_visible();
                    }
                }
            }

            // --- Match & Surround ---
            action::match_brackets => {
                crate::editors::edit_ops::match_bracket_jump(self);
            }
            action::select_textobject_around => {
                self.pending_textobject = Some(ned_text::textobject_mode::around);
                self.status_message = Some("match around: ".to_string());
            }
            action::select_textobject_inside => {
                self.pending_textobject = Some(ned_text::textobject_mode::inside);
                self.status_message = Some("match inside: ".to_string());
            }
            action::surround_add => {
                self.pending_surround = Some(crate::surround::surround_op::add);
                self.status_message = Some("surround add: ".to_string());
            }
            action::surround_replace => {
                self.pending_surround = Some(crate::surround::surround_op::replace_first);
                self.status_message = Some("surround replace (old): ".to_string());
            }
            action::surround_delete => {
                self.pending_surround = Some(crate::surround::surround_op::delete);
                self.status_message = Some("surround delete: ".to_string());
            }

            // --- Structural syntax navigation ---
            action::expand_selection => {
                crate::editors::edit_ops::expand_syntax_selection(self);
            }
            action::shrink_selection => {
                crate::editors::edit_ops::shrink_syntax_selection(self);
            }
            action::select_next_sibling => {
                crate::editors::edit_ops::select_next_syntax_sibling(self);
            }
            action::select_prev_sibling => {
                crate::editors::edit_ops::select_prev_syntax_sibling(self);
            }

            // --- Window / View management ---
            action::vsplit
            | action::hsplit
            | action::wclose
            | action::wonly
            | action::rotate_view
            | action::jump_view_left
            | action::jump_view_down
            | action::jump_view_up
            | action::jump_view_right => {
                self.status_message =
                    Some("splits not yet available in single-window mode".to_string());
            }

            action::no_op => {}
        }
    }

    /// Executes motion actions on the document's selection.
    fn execute_motion(&mut self, act: action) {
        let mov = if self.mode == mode::select {
            movement::extend
        } else {
            movement::move_
        };

        let count = self.count.take().unwrap_or(1);

        let doc = self.active_doc_mut();
        let buffer = doc.text();

        let new_sel = match act {
            action::move_char_left => apply_motion(buffer, doc.selection(), |r| {
                move_horizontally(buffer, r, direction::backward, count, mov)
            }),
            action::move_char_right => apply_motion(buffer, doc.selection(), |r| {
                move_horizontally(buffer, r, direction::forward, count, mov)
            }),
            action::move_line_up => apply_motion(buffer, doc.selection(), |r| {
                move_vertically(buffer, r, direction::backward, count, mov)
            }),
            action::move_line_down => apply_motion(buffer, doc.selection(), |r| {
                move_vertically(buffer, r, direction::forward, count, mov)
            }),
            action::move_next_word_start => apply_motion(buffer, doc.selection(), |r| {
                move_word_forward(buffer, r, count, mov)
            }),
            action::move_next_long_word_start => apply_motion(buffer, doc.selection(), |r| {
                move_long_word_forward(buffer, r, count, mov)
            }),
            action::move_prev_word_start => apply_motion(buffer, doc.selection(), |r| {
                move_word_backward(buffer, r, count, mov)
            }),
            action::move_prev_long_word_start => apply_motion(buffer, doc.selection(), |r| {
                move_long_word_backward(buffer, r, count, mov)
            }),
            action::move_next_word_end => apply_motion(buffer, doc.selection(), |r| {
                move_word_end(buffer, r, count, mov)
            }),
            action::move_next_long_word_end => apply_motion(buffer, doc.selection(), |r| {
                move_long_word_end(buffer, r, count, mov)
            }),
            action::goto_line_start => {
                apply_motion(buffer, doc.selection(), |r| move_line_start(buffer, r, mov))
            }
            action::goto_line_end => {
                apply_motion(buffer, doc.selection(), |r| move_line_end(buffer, r, mov))
            }
            action::goto_first_nonwhitespace => apply_motion(buffer, doc.selection(), |r| {
                move_first_non_whitespace(buffer, r, mov)
            }),
            action::goto_file_start => apply_motion(buffer, doc.selection(), |_| range::point(0)),
            action::goto_file_end => {
                apply_motion(buffer, doc.selection(), |_| range::point(buffer.char_len()))
            }
            action::goto_line => {
                let target_line = count.saturating_sub(1);
                apply_motion(buffer, doc.selection(), |_| {
                    let byte = buffer
                        .line_to_byte(target_line)
                        .unwrap_or_else(|| buffer.byte_len());
                    range::point(buffer.byte_to_char(byte))
                })
            }
            _ => doc.selection().clone(),
        };

        doc.set_selection(new_sel);
        self.ensure_active_cursor_visible();
    }

    /// Inserts a newline with auto-indentation and comment continuation.
    #[inline]
    pub fn insert_newline(&mut self) {
        edit_ops::insert_newline(self);
    }

    /// Inserts a single character literal at all active selection heads.
    #[inline]
    pub fn insert_char(&mut self, ch: char) {
        edit_ops::insert_char(self, ch);
    }

    /// Inserts a string at all active selection ranges.
    #[inline]
    pub fn insert_string(&mut self, s: &str) {
        edit_ops::insert_string(self, s);
    }

    /// Deletes character backward from each selection head.
    #[inline]
    fn delete_backward(&mut self) {
        edit_ops::delete_backward(self);
    }

    /// Deletes text covered by all selections and collapses selections to point cursors.
    #[inline]
    pub fn delete_selected_text(&mut self) {
        edit_ops::delete_selected_text(self);
    }

    /// Yanks text from active selection ranges into the chosen register.
    #[inline]
    fn yank_selection(&mut self) {
        edit_ops::yank_selection(self);
    }

    /// Deletes active selection with or without yanking.
    #[inline]
    fn delete_selection(&mut self, yank: bool) {
        edit_ops::delete_selection(self, yank);
    }

    /// Changes active selection with or without yanking.
    #[inline]
    fn change_selection(&mut self, yank: bool) {
        edit_ops::change_selection(self, yank);
    }

    /// Replaces active selection with a typed character literal.
    #[inline]
    pub fn replace_selection_with_char(&mut self, ch: char) {
        edit_ops::replace_selection_with_char(self, ch);
    }

    /// Replaces active selection with yanked text from chosen register.
    #[inline]
    fn replace_with_yanked(&mut self) {
        edit_ops::replace_with_yanked(self);
    }

    /// Pastes text from active register after or before cursor.
    #[inline]
    fn paste_register(&mut self, after: bool) {
        edit_ops::paste_register(self, after);
    }

    /// Applies casing transformations (lowercase, uppercase, switch case) to selections.
    #[inline]
    fn transform_selection_case(&mut self, act: action) {
        edit_ops::transform_selection_case(self, act);
    }

    /// Indents or unindents the lines intersecting selections.
    #[inline]
    fn indent_lines(&mut self, unindent: bool) {
        edit_ops::indent_lines(self, unindent);
    }

    /// Toggles line comments on active selections.
    #[inline]
    pub fn toggle_line_comment(&mut self) {
        indent_comment::toggle_line_comment(self);
    }

    /// Toggles block comments enclosing active selections.
    #[inline]
    pub fn toggle_block_comment(&mut self) {
        indent_comment::toggle_block_comment(self);
    }

    /// Rotates selected contents forward or backward.
    #[inline]
    fn rotate_selection_contents(&mut self, forward: bool, count: usize) {
        edit_ops::rotate_selection_contents(self, forward, count);
    }

    /// Aligns selections in columns.
    #[inline]
    fn align_selections(&mut self) {
        edit_ops::align_selections(self);
    }

    /// Joins selections with or without spaces.
    #[inline]
    fn join_selections(&mut self, with_space: bool) {
        edit_ops::join_selections(self, with_space);
    }

    /// Copies selection to line above or below.
    #[inline]
    fn copy_selection_on_line(&mut self, forward: bool, count: usize) {
        edit_ops::copy_selection_on_line(self, forward, count);
    }

    /// Shrinks selection to full line bounds.
    #[inline]
    fn shrink_to_line_bounds(&mut self) {
        edit_ops::shrink_to_line_bounds(self);
    }

    /// Selects sub-matches of regex within selections.
    #[inline]
    pub fn select_on_matches(&mut self, pattern: &str) {
        edit_ops::select_on_matches(self, pattern);
    }

    /// Splits selections on regex matches.
    #[inline]
    pub fn split_on_matches(&mut self, pattern: &str) {
        edit_ops::split_on_matches(self, pattern);
    }

    /// Keeps or removes selections matching regex.
    #[inline]
    pub fn keep_or_remove_matches(&mut self, pattern: &str, remove: bool) {
        edit_ops::keep_or_remove_matches(self, pattern, remove);
    }

    /// Jumps to next or previous match of `self.search_query` using `ned_regex`.
    pub fn jump_search(&mut self, forward: bool) {
        let query = match &self.search_query {
            Some(q) if !q.is_empty() => q.clone(),
            _ => return,
        };

        let re = match ned_regex::regex::smart_case(&query) {
            Ok(r) => r,
            Err(e) => {
                self.status_message = Some(format!("regex error: {e}"));
                return;
            }
        };

        let doc = self.active_doc_mut();
        let text_content = doc.text().to_string();
        let cursor_char = doc.selection().primary().cursor(doc.text());

        let matches: Vec<(usize, usize)> = re
            .find_iter(&text_content)
            .map(|m| {
                let start_char = doc.text().byte_to_char(m.start);
                let end_char = doc.text().byte_to_char(m.end);
                (start_char, end_char)
            })
            .collect();

        if matches.is_empty() {
            self.status_message = Some(format!("pattern not found: {query}"));
            return;
        }

        let target_match = if forward {
            matches
                .iter()
                .copied()
                .find(|&(start, _)| start > cursor_char)
                .unwrap_or(matches[0])
        } else {
            matches
                .iter()
                .copied()
                .rfind(|&(start, _)| start < cursor_char)
                .unwrap_or(*matches.last().unwrap_or(&(0, 0)))
        };

        doc.set_selection(selection::single(target_match.0, target_match.1));
        self.ensure_active_cursor_visible();
    }

    /// Executes an ex-command parsed from user input.
    #[inline]
    pub fn execute_command(&mut self, cmd_str: &str) {
        command_exec::execute_command(self, cmd_str);
    }

    /// Jumps to the start of the next diff hunk in the active document.
    pub fn goto_next_hunk(&mut self) {
        git_ops::goto_next_hunk(self);
    }

    /// Jumps to the start of the previous diff hunk in the active document.
    pub fn goto_prev_hunk(&mut self) {
        git_ops::goto_prev_hunk(self);
    }

    /// Refreshes the diff hunks for the specified document and updates editor status if capped.
    pub fn refresh_diff(&mut self, doc_id: document_id) {
        git_ops::refresh_diff(self, doc_id);
    }

    /// Document change notification (no-op retained for pure editor compatibility).
    #[inline]
    pub fn notify_lsp_did_change(&self, _doc_id: document_id) {}

    /// Document save notification (no-op retained for pure editor compatibility).
    #[inline]
    pub fn notify_lsp_did_save(&self, _doc_id: document_id) {}
}
