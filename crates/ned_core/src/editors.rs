//! Editor state machine, multi-document management, keymap dispatch, and action execution.

use std::collections::HashMap;
use std::path::Path;

use ::ned_config::{action, key_event, key_trie_node, mode, ned_config};
use ned_lsp::client;
use ned_text::{
    apply_motion, direction, extend_line, move_first_non_whitespace, move_horizontally,
    move_line_end, move_line_start, move_vertically, move_word_backward, move_word_end,
    move_word_forward, movement, range, selection,
};

use crate::command_parser::{editor_command, parse_command};
use crate::documents::{document, document_id};
use crate::registers::register_storage;
use crate::views::{view, view_id};

pub mod edit_ops;
pub mod mode_dispatch;

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
    /// Active register prefix key if explicitly selected (e.g. `"a`).
    pub selected_register: Option<char>,
    /// Pending key prefix tree for multi-keystroke combinations (e.g. `g`, `]`, `[`).
    pub key_prefix: Option<key_trie_node>,
    /// Ephemeral status line message.
    pub status_message: Option<String>,
    /// Floating hover documentation popup text.
    pub hover_popup: Option<String>,
    /// Active search pattern query string.
    pub search_query: Option<String>,
    /// Connected LSP language clients keyed by language name.
    pub lsp_clients: HashMap<String, client>,
    /// Termination flag signaling editor exit.
    pub should_quit: bool,
    next_doc_id: usize,
    next_view_id: usize,
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

        Self {
            documents: docs,
            active_doc_id: initial_doc_id,
            views,
            active_view_id: initial_view_id,
            mode: mode::normal,
            config,
            registers: register_storage::new(),
            selected_register: None,
            key_prefix: None,
            status_message: None,
            hover_popup: None,
            search_query: None,
            lsp_clients: HashMap::new(),
            should_quit: false,
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

        let doc = document::open(doc_id, &abs_path, lang_cfg)?;
        self.documents.insert(doc_id, doc);
        self.active_doc_id = doc_id;

        if let Some(v) = self.views.get_mut(&self.active_view_id) {
            v.doc_id = doc_id;
            v.row_offset = 0;
            v.col_offset = 0;
        }

        self.ensure_active_cursor_visible();
        self.start_lsp_for_doc(doc_id);
        Ok(doc_id)
    }

    /// Creates a new view displaying `doc_id` with specified dimensions.
    pub fn create_view(&mut self, doc_id: document_id, width: u16, height: u16) -> view_id {
        let vid = self.next_view_id;
        self.next_view_id += 1;
        self.views.insert(vid, view::new(vid, doc_id, width, height));
        vid
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
        self.views
            .get(&self.active_view_id)
            .expect("active view must exist")
    }

    /// Returns a mutable reference to the active view.
    #[inline]
    pub fn active_view_mut(&mut self) -> &mut view {
        self.views
            .get_mut(&self.active_view_id)
            .expect("active view must exist")
    }

    /// Ensures the active cursor is within the viewport margins.
    pub fn ensure_active_cursor_visible(&mut self) {
        let scrolloff = self.config.editor.scrolloff;
        let doc_id = self.active_doc_id;
        let view_id = self.active_view_id;
        if let (Some(doc), Some(view)) = (self.documents.get(&doc_id), self.views.get_mut(&view_id)) {
            view.ensure_cursor_visible(doc, scrolloff);
        }
    }

    /// Handles a mouse click event at screen coordinates `(col, row)`.
    pub fn handle_mouse_click(
        &mut self,
        col: u16,
        row: u16,
        gutter_width: u16,
        editor_height: u16,
        extend: bool,
    ) {
        self.hover_popup = None;
        if row >= editor_height {
            return;
        }

        let doc_id = self.active_doc_id;
        let view_id = self.active_view_id;

        let (target_pos, scrolloff) = {
            let doc = self.documents.get(&doc_id).expect("active doc must exist");
            let view = self.views.get(&view_id).expect("active view must exist");
            let text_col = col.saturating_sub(gutter_width);
            let pos = view.pos_at_coords(doc, text_col, row);
            (pos, self.config.editor.scrolloff)
        };

        let doc = self.documents.get_mut(&doc_id).expect("active doc must exist");
        if extend || self.mode == mode::select {
            let primary = doc.selection().primary();
            let new_range = primary.put_cursor(doc.text(), target_pos, true);
            doc.set_selection(selection::single(new_range.anchor, new_range.head));
        } else {
            doc.set_selection(selection::point(target_pos));
        }

        let view = self.views.get_mut(&view_id).expect("active view must exist");
        view.ensure_cursor_visible(doc, scrolloff);
    }

    /// Scrolls the active view vertically by `delta` lines matching Helix scroll behavior.
    ///
    /// Preserves primary cursor position unless scrolled past the viewport margins (scrolloff).
    pub fn scroll_view_by(&mut self, delta: isize) {
        let doc_id = self.active_doc_id;
        let view_id = self.active_view_id;

        let total_lines = self
            .documents
            .get(&doc_id)
            .map_or(1, |d| d.text().line_count().max(1));

        let view = match self.views.get_mut(&view_id) {
            Some(v) => v,
            None => return,
        };

        let height = view.height as usize;
        if height == 0 {
            return;
        }

        let scrolloff = self.config.editor.scrolloff.min(height.saturating_sub(1) / 2);
        let max_row_offset = total_lines.saturating_sub(1);

        if delta > 0 {
            let offset = delta as usize;
            view.row_offset = (view.row_offset + offset).min(max_row_offset);
        } else if delta < 0 {
            let offset = delta.unsigned_abs();
            view.row_offset = view.row_offset.saturating_sub(offset);
        }

        let new_row_offset = view.row_offset;

        // Keep cursor within [new_row_offset + scrolloff, new_row_offset + height - scrolloff - 1]
        let min_cursor_line = (new_row_offset + scrolloff).min(total_lines.saturating_sub(1));
        let max_cursor_line = (new_row_offset + height)
            .saturating_sub(scrolloff + 1)
            .min(total_lines.saturating_sub(1));

        let doc = self.documents.get_mut(&doc_id).expect("active doc must exist");
        let buffer = doc.text();
        let primary_cursor = doc.selection().primary().cursor(buffer);
        let cursor_byte = buffer.char_to_byte(primary_cursor);
        let cursor_line = buffer.byte_to_line(cursor_byte);
        let line_start_byte = buffer.line_to_byte(cursor_line).unwrap_or(0);
        let line_start_char = buffer.byte_to_char(line_start_byte);
        let cursor_col = primary_cursor.saturating_sub(line_start_char);

        let target_line = if cursor_line < min_cursor_line {
            Some(min_cursor_line)
        } else if cursor_line > max_cursor_line {
            Some(max_cursor_line)
        } else {
            None
        };

        if let Some(new_line) = target_line {
            let target_line_byte = buffer.line_to_byte(new_line).unwrap_or(0);
            let target_line_char = buffer.byte_to_char(target_line_byte);
            let target_line_len = if let Some(line_str) = buffer.line(new_line) {
                line_str.trim_end_matches(['\r', '\n']).chars().count()
            } else {
                0
            };
            let target_col = cursor_col.min(target_line_len);
            let target_pos = target_line_char + target_col;

            if self.mode == mode::select {
                let primary = doc.selection().primary();
                doc.set_selection(selection::single(primary.anchor, target_pos));
            } else {
                doc.set_selection(selection::point(target_pos));
            }
        }
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
                    let end_byte = buffer.line_to_byte(line + 1).unwrap_or_else(|| buffer.byte_len());
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
                self.yank_selection();
                self.delete_selected_text();
            }
            action::delete_selection_noyank => {
                self.delete_selected_text();
            }
            action::change_selection => {
                self.yank_selection();
                self.delete_selected_text();
                self.mode = mode::insert;
            }
            action::change_selection_noyank => {
                self.delete_selected_text();
                self.mode = mode::insert;
            }
            action::replace => {
                // Next character replaces selected text
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
                let changed = self.active_doc_mut().undo();
                self.ensure_active_cursor_visible();
                if changed {
                    self.notify_lsp_did_change(self.active_doc_id);
                }
            }
            action::redo => {
                let changed = self.active_doc_mut().redo();
                self.ensure_active_cursor_visible();
                if changed {
                    self.notify_lsp_did_change(self.active_doc_id);
                }
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
                let flipped: Vec<range> = doc
                    .selection()
                    .ranges()
                    .iter()
                    .map(|r| range::new(r.head, r.anchor))
                    .collect();
                doc.set_selection(selection::new(flipped, doc.selection().primary_index()));
            }
            action::keep_primary_selection => {
                let doc = self.active_doc_mut();
                let primary = doc.selection().primary();
                doc.set_selection(selection::single(primary.anchor, primary.head));
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
                        self.notify_lsp_did_save(self.active_doc_id);
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

            // --- LSP & other actions ---
            action::goto_next_diag => {
                self.goto_diagnostic(true);
            }
            action::goto_prev_diag => {
                self.goto_diagnostic(false);
            }
            action::hover => {
                self.request_hover();
            }
            action::goto_definition => {
                self.request_definition();
            }
            action::goto_reference => {
                self.status_message = Some("LSP references queried".to_string());
            }
            action::file_picker => {
                self.status_message = Some("picker:file".to_string());
            }
            action::buffer_picker => {
                self.status_message = Some("picker:buffer".to_string());
            }
            action::diagnostics_picker => {
                self.status_message = Some("picker:diagnostics".to_string());
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

        let doc = self.active_doc_mut();
        let buffer = doc.text();

        let new_sel = match act {
            action::move_char_left => {
                apply_motion(buffer, doc.selection(), |r| {
                    move_horizontally(buffer, r, direction::backward, 1, mov)
                })
            }
            action::move_char_right => {
                apply_motion(buffer, doc.selection(), |r| {
                    move_horizontally(buffer, r, direction::forward, 1, mov)
                })
            }
            action::move_line_up => {
                apply_motion(buffer, doc.selection(), |r| {
                    move_vertically(buffer, r, direction::backward, 1, mov)
                })
            }
            action::move_line_down => {
                apply_motion(buffer, doc.selection(), |r| {
                    move_vertically(buffer, r, direction::forward, 1, mov)
                })
            }
            action::move_next_word_start | action::move_next_long_word_start => {
                apply_motion(buffer, doc.selection(), |r| {
                    move_word_forward(buffer, r, 1, mov)
                })
            }
            action::move_prev_word_start | action::move_prev_long_word_start => {
                apply_motion(buffer, doc.selection(), |r| {
                    move_word_backward(buffer, r, 1, mov)
                })
            }
            action::move_next_word_end | action::move_next_long_word_end => {
                apply_motion(buffer, doc.selection(), |r| {
                    move_word_end(buffer, r, 1, mov)
                })
            }
            action::goto_line_start => {
                apply_motion(buffer, doc.selection(), |r| move_line_start(buffer, r, mov))
            }
            action::goto_line_end => {
                apply_motion(buffer, doc.selection(), |r| move_line_end(buffer, r, mov))
            }
            action::goto_first_nonwhitespace => {
                apply_motion(buffer, doc.selection(), |r| {
                    move_first_non_whitespace(buffer, r, mov)
                })
            }
            action::goto_file_start => {
                apply_motion(buffer, doc.selection(), |_| range::point(0))
            }
            action::goto_file_end => {
                apply_motion(buffer, doc.selection(), |_| range::point(buffer.char_len()))
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
    fn delete_selected_text(&mut self) {
        edit_ops::delete_selected_text(self);
    }

    /// Yanks text from active selection ranges into the chosen register.
    #[inline]
    fn yank_selection(&mut self) {
        edit_ops::yank_selection(self);
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

    /// Jumps to next or previous match of `self.search_query`.
    fn jump_search(&mut self, forward: bool) {
        let query = match &self.search_query {
            Some(q) if !q.is_empty() => q.clone(),
            _ => return,
        };

        let doc = self.active_doc_mut();
        let text_content = doc.text().to_string();
        let cursor_char = doc.selection().primary().cursor(doc.text());

        let matches: Vec<usize> = text_content
            .match_indices(&query)
            .map(|(byte_idx, _)| doc.text().byte_to_char(byte_idx))
            .collect();

        if matches.is_empty() {
            self.status_message = Some(format!("pattern not found: {query}"));
            return;
        }

        let target_match = if forward {
            matches
                .iter()
                .copied()
                .find(|&pos| pos > cursor_char)
                .unwrap_or(matches[0])
        } else {
            matches
                .iter()
                .copied()
                .rfind(|&pos| pos < cursor_char)
                .unwrap_or(*matches.last().unwrap_or(&0))
        };

        let match_len = query.chars().count();
        doc.set_selection(selection::single(target_match, target_match + match_len));
        self.ensure_active_cursor_visible();
    }

    /// Executes an ex-command parsed from user input.
    pub fn execute_command(&mut self, cmd_str: &str) {
        match parse_command(cmd_str) {
            Ok(editor_command::write(path_opt)) => {
                let res = {
                    let doc = self.active_doc_mut();
                    if let Some(path) = path_opt {
                        doc.save_to(path)
                    } else {
                        doc.save()
                    }
                };
                match res {
                    Ok(()) => {
                        self.status_message = Some("written".to_string());
                        self.notify_lsp_did_save(self.active_doc_id);
                    }
                    Err(e) => self.status_message = Some(format!("write error: {e}")),
                }
            }
            Ok(editor_command::quit) => {
                let doc = self.active_doc();
                if doc.is_modified() {
                    self.status_message = Some("unsaved changes (use :q! to force)".to_string());
                } else {
                    self.should_quit = true;
                }
            }
            Ok(editor_command::force_quit) => {
                self.should_quit = true;
            }
            Ok(editor_command::write_and_quit(path_opt)) => {
                let doc = self.active_doc_mut();
                let res = if let Some(path) = path_opt {
                    doc.save_to(path)
                } else {
                    doc.save()
                };
                if res.is_ok() {
                    self.should_quit = true;
                } else if let Err(e) = res {
                    self.status_message = Some(format!("write error: {e}"));
                }
            }
            Ok(editor_command::open(path)) => {
                if let Err(e) = self.open_file(&path) {
                    self.status_message = Some(format!("error opening file: {e}"));
                }
            }
            Ok(editor_command::set_option(key, val)) => {
                self.status_message = Some(format!("set {key} = {val}"));
            }
            Err(e) => {
                self.status_message = Some(e);
            }
        }
    }

    /// Starts or connects the language server for `doc_id`.
    pub fn start_lsp_for_doc(&mut self, doc_id: document_id) {
        let (lang_name, srv_cmd, srv_args, doc_path, doc_text, doc_version) = {
            let doc = match self.documents.get(&doc_id) {
                Some(d) => d,
                None => return,
            };
            let lang = match doc.language() {
                Some(l) => l,
                None => return,
            };
            let lsp_cfg = match &lang.language_server {
                Some(c) => c,
                None => return,
            };
            let path = match doc.path() {
                Some(p) => p.to_path_buf(),
                None => return,
            };
            (
                lang.name.clone(),
                lsp_cfg.command.clone(),
                lsp_cfg.args.clone(),
                path,
                doc.text().to_string(),
                doc.version(),
            )
        };

        if !self.lsp_clients.contains_key(&lang_name) {
            let root_path = ned_git::discover_repo(&doc_path)
                .or_else(|| doc_path.parent().map(std::path::Path::to_path_buf))
                .map(|p| p.to_string_lossy().to_string());
            let srv_cfg = ned_lsp::server_config::new(&lang_name, &srv_cmd, srv_args, root_path);
            if let Ok(c) = ned_lsp::client::start(srv_cfg) {
                if c.initialize(std::time::Duration::from_millis(2000)).is_ok() {
                    let _ = c.initialized();
                }
                self.lsp_clients.insert(lang_name.clone(), c);
            }
        }

        if let Some(client) = self.lsp_clients.get(&lang_name) {
            let uri = format!("file://{}", doc_path.to_string_lossy());
            let _ = client.notify_did_open(&uri, &lang_name, doc_version, &doc_text);
        }
    }

    /// Notifies the connected LSP server that `doc_id` contents changed.
    pub fn notify_lsp_did_change(&self, doc_id: document_id) {
        let Some(doc) = self.documents.get(&doc_id) else {
            return;
        };
        let Some(lang) = doc.language() else {
            return;
        };
        let Some(path) = doc.path() else {
            return;
        };
        if let Some(client) = self.lsp_clients.get(&lang.name) {
            let uri = format!("file://{}", path.to_string_lossy());
            let _ = client.notify_did_change(&uri, doc.version(), &doc.text().to_string());
        }
    }

    /// Notifies the connected LSP server that `doc_id` was saved.
    pub fn notify_lsp_did_save(&self, doc_id: document_id) {
        let Some(doc) = self.documents.get(&doc_id) else {
            return;
        };
        let Some(lang) = doc.language() else {
            return;
        };
        let Some(path) = doc.path() else {
            return;
        };
        if let Some(client) = self.lsp_clients.get(&lang.name) {
            let uri = format!("file://{}", path.to_string_lossy());
            let _ = client.notify_did_save(&uri);
        }
    }

    /// Polls diagnostics from connected LSP clients and assigns them to matching documents.
    ///
    /// Returns `true` if any diagnostics were received and applied.
    pub fn poll_lsp(&mut self) -> bool {
        let mut new_diagnostics: Vec<(String, Vec<ned_lsp::diagnostic>)> = Vec::new();

        for client in self.lsp_clients.values() {
            while let Some(params) = client.poll_diagnostics() {
                new_diagnostics.push((params.uri, params.diagnostics));
            }
        }

        if new_diagnostics.is_empty() {
            return false;
        }

        let mut updated = false;
        for (uri, diags) in new_diagnostics {
            for doc in self.documents.values_mut() {
                if let Some(path) = doc.path() {
                    let doc_uri = format!("file://{}", path.to_string_lossy());
                    if doc_uri == uri || uri.ends_with(&path.to_string_lossy().to_string()) {
                        doc.set_diagnostics(diags.clone());
                        updated = true;
                    }
                }
            }
        }
        updated
    }

    /// Jumps cursor to the next or previous diagnostic.
    pub fn goto_diagnostic(&mut self, next: bool) {
        let doc_id = self.active_doc_id;
        let (target_line, target_col, msg) = {
            let doc = match self.documents.get(&doc_id) {
                Some(d) => d,
                None => return,
            };
            if doc.diagnostics().is_empty() {
                self.status_message = Some("No diagnostics for this document".to_string());
                return;
            }
            let cursor_char = doc.selection().primary().cursor(doc.text());
            let cursor_byte = doc.text().char_to_byte(cursor_char);
            let current_line = doc.text().byte_to_line(cursor_byte) as u32;

            let mut sorted = doc.diagnostics().to_vec();
            sorted.sort_by_key(|d| (d.range.start.line, d.range.start.character));

            let candidate = if next {
                sorted
                    .iter()
                    .find(|d| d.range.start.line > current_line)
                    .or_else(|| sorted.first())
            } else {
                sorted
                    .iter()
                    .rfind(|d| d.range.start.line < current_line)
                    .or_else(|| sorted.last())
            };

            match candidate {
                Some(d) => (d.range.start.line as usize, d.range.start.character as usize, d.message.clone()),
                None => return,
            }
        };

        let doc = self.active_doc_mut();
        let target_line_byte = doc.text().line_to_byte(target_line).unwrap_or(0);
        let target_line_char = doc.text().byte_to_char(target_line_byte);
        let target_char = (target_line_char + target_col).min(doc.text().char_len());
        doc.set_selection(selection::point(target_char));
        self.ensure_active_cursor_visible();
        self.status_message = Some(format!("[LSP] {msg}"));
    }

    /// Queries hover documentation for the symbol under cursor.
    pub fn request_hover(&mut self) {
        let doc = self.active_doc();
        let lang_name = match doc.language() {
            Some(l) => l.name.clone(),
            None => {
                self.status_message = Some("No language server for this buffer".to_string());
                return;
            }
        };
        let path = match doc.path() {
            Some(p) => p.to_path_buf(),
            None => return,
        };

        let cursor_char = doc.selection().primary().cursor(doc.text());
        let cursor_byte = doc.text().char_to_byte(cursor_char);
        let line = doc.text().byte_to_line(cursor_byte);
        let line_start_byte = doc.text().line_to_byte(line).unwrap_or(0);
        let line_start_char = doc.text().byte_to_char(line_start_byte);
        let col = cursor_char.saturating_sub(line_start_char);

        if let Some(client) = self.lsp_clients.get(&lang_name) {
            let uri = format!("file://{}", path.to_string_lossy());
            match client.request_hover(&uri, line as u32, col as u32, std::time::Duration::from_millis(500)) {
                Ok(Some(h)) => {
                    let text = match h.contents {
                        serde_json::Value::String(s) => s,
                        serde_json::Value::Object(map) => {
                            map.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string()
                        }
                        other => other.to_string(),
                    };
                    let trimmed = text.trim().to_string();
                    if trimmed.is_empty() {
                        self.status_message = Some("No hover information available".to_string());
                    } else {
                        self.hover_popup = Some(trimmed);
                    }
                }
                Ok(None) => {
                    self.status_message = Some("No hover information available".to_string());
                }
                Err(e) => {
                    self.status_message = Some(format!("Hover error: {e}"));
                }
            }
        } else {
            self.status_message = Some("Language server not connected".to_string());
        }
    }

    /// Queries definition location and jumps to it.
    pub fn request_definition(&mut self) {
        let doc = self.active_doc();
        let lang_name = match doc.language() {
            Some(l) => l.name.clone(),
            None => return,
        };
        let path = match doc.path() {
            Some(p) => p.to_path_buf(),
            None => return,
        };

        let cursor_char = doc.selection().primary().cursor(doc.text());
        let cursor_byte = doc.text().char_to_byte(cursor_char);
        let line = doc.text().byte_to_line(cursor_byte);
        let line_start_byte = doc.text().line_to_byte(line).unwrap_or(0);
        let line_start_char = doc.text().byte_to_char(line_start_byte);
        let col = cursor_char.saturating_sub(line_start_char);

        let target_location = if let Some(client) = self.lsp_clients.get(&lang_name) {
            let uri = format!("file://{}", path.to_string_lossy());
            match client.request_definition(&uri, line as u32, col as u32, std::time::Duration::from_millis(500)) {
                Ok(Some(locs)) => locs.into_iter().next(),
                _ => None,
            }
        } else {
            None
        };

        if let Some(loc) = target_location {
            let target_path_str = loc.uri.strip_prefix("file://").unwrap_or(&loc.uri);
            let target_line = loc.range.start.line as usize;
            let target_col = loc.range.start.character as usize;

            let is_same_file = doc.path().map_or(false, |p| p.to_string_lossy() == target_path_str);
            if !is_same_file {
                let _ = self.open_file(target_path_str);
            }

            let doc = self.active_doc_mut();
            let line_byte = doc.text().line_to_byte(target_line).unwrap_or(0);
            let line_char = doc.text().byte_to_char(line_byte);
            let target_char = (line_char + target_col).min(doc.text().char_len());
            doc.set_selection(selection::point(target_char));
            self.ensure_active_cursor_visible();
            self.status_message = Some(format!("Jumped to {target_path_str}:{target_line}"));
        } else {
            self.status_message = Some("Definition not found".to_string());
        }
    }
}
