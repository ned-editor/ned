//! Editor state machine, multi-document management, keymap dispatch, and action execution.

use std::collections::HashMap;
use std::path::Path;

use ::ned_config::{
    action, key_code, key_event, key_trie, key_trie_node, mode, ned_config,
};
use ned_lsp::client;
use ned_text::{
    apply_motion, change_set, direction, extend_line, move_first_non_whitespace,
    move_horizontally, move_line_end, move_line_start, move_vertically, move_word_backward,
    move_word_end, move_word_forward, movement, prev_grapheme_boundary, range, selection,
};

use crate::command_parser::{editor_command, parse_command};
use crate::documents::{document, document_id};
use crate::registers::register_storage;
use crate::views::{view, view_id};

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
        let lang_cfg = self.config.languages.detect_by_path(path_ref).cloned();

        let doc_id = self.next_doc_id;
        self.next_doc_id += 1;

        let doc = document::open(doc_id, path_ref, lang_cfg)?;
        self.documents.insert(doc_id, doc);
        self.active_doc_id = doc_id;

        if let Some(v) = self.views.get_mut(&self.active_view_id) {
            v.doc_id = doc_id;
            v.row_offset = 0;
            v.col_offset = 0;
        }

        self.ensure_active_cursor_visible();
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

    /// Processes an incoming keyboard key event.
    ///
    /// Returns `true` if the event was handled and may require redraw.
    pub fn handle_key(&mut self, ev: key_event) -> bool {
        if self.mode == mode::insert {
            // Check insert mode keybindings first (e.g. Esc -> normal_mode, Backspace)
            let matched_entry = self
                .config
                .keys
                .get(&mode::insert)
                .and_then(|trie| trie.get(&ev).cloned());

            if let Some(entry) = matched_entry {
                match entry {
                    key_trie::action(act) => {
                        self.dispatch(act);
                        return true;
                    }
                    key_trie::sequence(acts) => {
                        for act in acts {
                            self.dispatch(act);
                        }
                        return true;
                    }
                    key_trie::node(_) => {}
                }
            }

            // Insert character literal if key is a character without Ctrl/Alt modifiers
            if !ev.modifiers.ctrl && !ev.modifiers.alt {
                if let key_code::char(ch) = ev.code {
                    self.insert_char(ch);
                    return true;
                }
            }
            return false;
        }

        // Normal or Select mode dispatch
        if let Some(prefix_node) = self.key_prefix.take() {
            if let Some(entry) = prefix_node.get(&ev) {
                match entry {
                    key_trie::action(act) => {
                        self.dispatch(*act);
                    }
                    key_trie::sequence(acts) => {
                        for act in acts {
                            self.dispatch(*act);
                        }
                    }
                    key_trie::node(sub_node) => {
                        self.key_prefix = Some(sub_node.clone());
                    }
                }
                return true;
            }
            // Key not in prefix trie: prefix cancelled
            return true;
        }

        // Top-level modal keymap lookup
        let matched_entry = self
            .config
            .keys
            .get(&self.mode)
            .and_then(|trie| trie.get(&ev).cloned());

        if let Some(entry) = matched_entry {
            match entry {
                key_trie::action(act) => {
                    self.dispatch(act);
                }
                key_trie::sequence(acts) => {
                    for act in acts {
                        self.dispatch(act);
                    }
                }
                key_trie::node(sub_node) => {
                    self.key_prefix = Some(sub_node);
                }
            }
            return true;
        }

        // Register prefix shortcut (`"`)
        if ev.code == key_code::char('"') && !ev.modifiers.ctrl && !ev.modifiers.alt {
            // Note: register selection continuation can be handled on subsequent key
            return true;
        }

        false
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
                let doc = self.active_doc_mut();
                doc.undo();
                self.ensure_active_cursor_visible();
            }
            action::redo => {
                let doc = self.active_doc_mut();
                doc.redo();
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
                let doc = self.active_doc_mut();
                match doc.save() {
                    Ok(()) => self.status_message = Some("written".to_string()),
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
                self.insert_char('\n');
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
            action::goto_definition
            | action::goto_reference
            | action::hover
            | action::goto_next_diag
            | action::goto_prev_diag
            | action::no_op => {}
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

    /// Inserts a single character literal at all active selection heads.
    pub fn insert_char(&mut self, ch: char) {
        let mut s = String::new();
        s.push(ch);
        self.insert_string(&s);
    }

    /// Inserts a string at all active selection ranges.
    pub fn insert_string(&mut self, s: &str) {
        let doc = self.active_doc_mut();
        let doc_len = doc.text().char_len();
        let mut cs = change_set::new(doc_len);
        let mut last_pos = 0;

        for r in doc.selection().ranges() {
            let from = r.from();
            let to = r.to();
            if from > last_pos {
                cs.retain(from - last_pos);
            }
            if to > from {
                cs.delete(to - from);
            }
            cs.insert(s);
            last_pos = to;
        }
        if last_pos < doc_len {
            cs.retain(doc_len - last_pos);
        }

        let new_sel = cs.map_selection(doc.selection().clone());
        doc.apply(cs, Some(new_sel));
        self.ensure_active_cursor_visible();
    }

    /// Deletes character backward from each selection head.
    fn delete_backward(&mut self) {
        let doc = self.active_doc_mut();
        let doc_len = doc.text().char_len();
        let mut cs = change_set::new(doc_len);
        let mut last_pos = 0;

        for r in doc.selection().ranges() {
            let from = r.from();
            let to = r.to();
            if to > from {
                if from > last_pos {
                    cs.retain(from - last_pos);
                }
                cs.delete(to - from);
                last_pos = to;
            } else if from > 0 {
                let prev = prev_grapheme_boundary(doc.text(), from);
                if prev > last_pos {
                    cs.retain(prev - last_pos);
                }
                cs.delete(from - prev);
                last_pos = from;
            }
        }
        if last_pos < doc_len {
            cs.retain(doc_len - last_pos);
        }

        let new_sel = cs.map_selection(doc.selection().clone());
        doc.apply(cs, Some(new_sel));
        self.ensure_active_cursor_visible();
    }

    /// Deletes text covered by all selections and collapses selections to point cursors.
    fn delete_selected_text(&mut self) {
        let doc = self.active_doc_mut();
        let doc_len = doc.text().char_len();
        let mut cs = change_set::new(doc_len);
        let mut last_pos = 0;

        for r in doc.selection().ranges() {
            let from = r.from();
            let to = r.to().max(from + 1).min(doc_len);
            if from > last_pos {
                cs.retain(from - last_pos);
            }
            if to > from {
                cs.delete(to - from);
            }
            last_pos = to;
        }
        if last_pos < doc_len {
            cs.retain(doc_len - last_pos);
        }

        let new_sel = cs.map_selection(doc.selection().clone());
        doc.apply(cs, Some(new_sel));
        self.ensure_active_cursor_visible();
    }

    /// Yanks text from active selection ranges into the chosen register.
    fn yank_selection(&mut self) {
        let reg = self.selected_register.take().unwrap_or('"');
        let doc = self.active_doc();
        let mut values = Vec::new();

        for r in doc.selection().ranges() {
            let from = r.from();
            let to = r.to().max(from + 1).min(doc.text().char_len());
            values.push(doc.text().slice_chars(from..to));
        }

        self.registers.write(reg, values);
    }

    /// Pastes text from active register after or before cursor.
    fn paste_register(&mut self, after: bool) {
        let reg = self.selected_register.take().unwrap_or('"');
        let strings = match self.registers.read(reg) {
            Some(vals) => vals.to_vec(),
            None => return,
        };
        if strings.is_empty() {
            return;
        }

        let text_to_paste = strings.join("\n");
        if after {
            let doc = self.active_doc_mut();
            let sel = apply_motion(doc.text(), doc.selection(), |r| {
                move_horizontally(doc.text(), r, direction::forward, 1, movement::move_)
            });
            doc.set_selection(sel);
        }
        self.insert_string(&text_to_paste);
    }

    /// Applies casing transformations (lowercase, uppercase, switch case) to selections.
    fn transform_selection_case(&mut self, act: action) {
        let doc = self.active_doc_mut();
        let doc_len = doc.text().char_len();
        let mut cs = change_set::new(doc_len);
        let mut last_pos = 0;

        for r in doc.selection().ranges() {
            let from = r.from();
            let to = r.to().max(from + 1).min(doc_len);
            if from > last_pos {
                cs.retain(from - last_pos);
            }
            let original = doc.text().slice_chars(from..to);
            let transformed: String = original
                .chars()
                .map(|ch| match act {
                    action::switch_case => {
                        if ch.is_lowercase() {
                            ch.to_uppercase().collect::<String>()
                        } else {
                            ch.to_lowercase().collect::<String>()
                        }
                    }
                    action::switch_to_lowercase => ch.to_lowercase().collect::<String>(),
                    action::switch_to_uppercase => ch.to_uppercase().collect::<String>(),
                    _ => ch.to_string(),
                })
                .collect();
            cs.delete(to - from);
            cs.insert(&transformed);
            last_pos = to;
        }
        if last_pos < doc_len {
            cs.retain(doc_len - last_pos);
        }

        let new_sel = cs.map_selection(doc.selection().clone());
        doc.apply(cs, Some(new_sel));
        self.ensure_active_cursor_visible();
    }

    /// Indents or unindents the lines intersecting selections.
    fn indent_lines(&mut self, unindent: bool) {
        let tab_width = self.config.editor.tab_width;
        let tab_str = " ".repeat(tab_width);
        let doc = self.active_doc_mut();
        let buffer = doc.text();
        let mut lines = Vec::new();

        for r in doc.selection().ranges() {
            let start_line = buffer.byte_to_line(buffer.char_to_byte(r.from()));
            let end_line = buffer.byte_to_line(buffer.char_to_byte(r.to()));
            for line in start_line..=end_line {
                if !lines.contains(&line) {
                    lines.push(line);
                }
            }
        }
        lines.sort_unstable();

        let doc_len = buffer.char_len();
        let mut cs = change_set::new(doc_len);
        let mut last_pos = 0;

        for line in lines {
            let line_start_byte = buffer.line_to_byte(line).unwrap_or(0);
            let line_start_char = buffer.byte_to_char(line_start_byte);
            if line_start_char > last_pos {
                cs.retain(line_start_char - last_pos);
            }

            if unindent {
                let mut spaces_to_remove = 0;
                for i in 0..tab_width {
                    if let Some(ch) = buffer.char_at(line_start_char + i) {
                        if ch == ' ' {
                            spaces_to_remove += 1;
                        } else if ch == '\t' && spaces_to_remove == 0 {
                            spaces_to_remove = 1;
                            break;
                        } else {
                            break;
                        }
                    }
                }
                if spaces_to_remove > 0 {
                    cs.delete(spaces_to_remove);
                }
                last_pos = line_start_char + spaces_to_remove;
            } else {
                cs.insert(&tab_str);
                last_pos = line_start_char;
            }
        }
        if last_pos < doc_len {
            cs.retain(doc_len - last_pos);
        }

        let new_sel = cs.map_selection(doc.selection().clone());
        doc.apply(cs, Some(new_sel));
        self.ensure_active_cursor_visible();
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
                let doc = self.active_doc_mut();
                let res = if let Some(path) = path_opt {
                    doc.save_to(path)
                } else {
                    doc.save()
                };
                match res {
                    Ok(()) => self.status_message = Some("written".to_string()),
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
}
