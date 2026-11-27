//! Document editing operations: insert, delete, change, yank, paste, casing, and indentation.

use ned_config::action;
use ned_text::{
    apply_motion, change_set, direction, move_horizontally, movement, prev_grapheme_boundary,
};

use crate::editors::editor;

/// Inserts a newline with auto-indentation and comment continuation.
pub fn insert_newline(ed: &mut editor) {
    let (indent, comment_prefix, is_empty_comment) = {
        let doc = ed.active_doc();
        let cursor_char = doc.selection().primary().cursor(doc.text());
        let cursor_byte = doc.text().char_to_byte(cursor_char);
        let line_idx = doc.text().byte_to_line(cursor_byte);
        let line_str = doc.text().line(line_idx).unwrap_or_default();
        let line_start_byte = doc.text().line_to_byte(line_idx).unwrap_or(0);
        let col_byte = cursor_byte.saturating_sub(line_start_byte);
        let prefix_before = &line_str[..col_byte.min(line_str.len())];

        let indent: String = line_str
            .chars()
            .take_while(|&c| c == ' ' || c == '\t')
            .collect();
        let trimmed = prefix_before.trim();

        if trimmed == "//" || trimmed == "#" || trimmed == "*" {
            (indent, None, true)
        } else if trimmed.starts_with("//") {
            (indent, Some("// "), false)
        } else if trimmed.starts_with('#') {
            (indent, Some("# "), false)
        } else if trimmed.starts_with("/*") || trimmed.starts_with('*') {
            (indent, Some("* "), false)
        } else {
            (indent, None, false)
        }
    };

    if is_empty_comment {
        let mut newline_str = String::from("\n");
        newline_str.push_str(&indent);
        delete_backward(ed);
        insert_string(ed, &newline_str);
    } else {
        let mut newline_str = String::from("\n");
        newline_str.push_str(&indent);
        if let Some(cp) = comment_prefix {
            newline_str.push_str(cp);
        }
        insert_string(ed, &newline_str);
    }
}

/// Inserts a single character literal at all active selection heads.
pub fn insert_char(ed: &mut editor, ch: char) {
    let mut s = String::new();
    s.push(ch);
    insert_string(ed, &s);
}

/// Inserts a string at all active selection ranges.
pub fn insert_string(ed: &mut editor, s: &str) {
    let doc_id = ed.active_doc_id;
    let doc = ed.active_doc_mut();
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
    ed.ensure_active_cursor_visible();
    ed.notify_lsp_did_change(doc_id);
}

/// Deletes character backward from each selection head.
pub fn delete_backward(ed: &mut editor) {
    let doc_id = ed.active_doc_id;
    let doc = ed.active_doc_mut();
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
    ed.ensure_active_cursor_visible();
    ed.notify_lsp_did_change(doc_id);
}

/// Deletes text covered by all selections and collapses selections to point cursors.
pub fn delete_selected_text(ed: &mut editor) {
    let doc_id = ed.active_doc_id;
    let doc = ed.active_doc_mut();
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
    ed.ensure_active_cursor_visible();
    ed.notify_lsp_did_change(doc_id);
}

/// Yanks text from active selection ranges into the chosen register.
pub fn yank_selection(ed: &mut editor) {
    let reg = ed.selected_register.take().unwrap_or('"');
    let doc = ed.active_doc();
    let mut values = Vec::new();

    for r in doc.selection().ranges() {
        let from = r.from();
        let to = r.to().max(from + 1).min(doc.text().char_len());
        values.push(doc.text().slice_chars(from..to));
    }

    ed.registers.write(reg, values);
}

/// Pastes text from active register after or before cursor.
pub fn paste_register(ed: &mut editor, after: bool) {
    let reg = ed.selected_register.take().unwrap_or('"');
    let strings = match ed.registers.read(reg) {
        Some(vals) => vals.to_vec(),
        None => return,
    };
    if strings.is_empty() {
        return;
    }

    let text_to_paste = strings.join("\n");
    if after {
        let doc = ed.active_doc_mut();
        let sel = apply_motion(doc.text(), doc.selection(), |r| {
            move_horizontally(doc.text(), r, direction::forward, 1, movement::move_)
        });
        doc.set_selection(sel);
    }
    insert_string(ed, &text_to_paste);
}

/// Applies casing transformations (lowercase, uppercase, switch case) to selections.
pub fn transform_selection_case(ed: &mut editor, act: action) {
    let doc = ed.active_doc_mut();
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
    ed.ensure_active_cursor_visible();
}

/// Indents or unindents the lines intersecting selections.
pub fn indent_lines(ed: &mut editor, unindent: bool) {
    let tab_width = ed.config.editor.tab_width;
    let tab_str = " ".repeat(tab_width);
    let doc = ed.active_doc_mut();
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
    ed.ensure_active_cursor_visible();
}
