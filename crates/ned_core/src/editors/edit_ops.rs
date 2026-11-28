//! Document editing operations: insert, delete, change, yank, paste, casing, and indentation.

use ned_config::action;
use ned_text::{change_set, prev_grapheme_boundary};

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

/// Reads values from register `reg` taking dynamic editor context into account
/// (e.g. `.` for current selections, `#` for selection indices, `%` for file path).
#[must_use]
pub fn read_register(ed: &editor, reg: char) -> Option<Vec<String>> {
    match reg {
        '_' => None,
        '.' => {
            let doc = ed.active_doc();
            let mut vals = Vec::new();
            for r in doc.selection().ranges() {
                let from = r.from();
                let to = r.to().max(from + 1).min(doc.text().char_len());
                vals.push(doc.text().slice_chars(from..to));
            }
            Some(vals)
        }
        '#' => {
            let doc = ed.active_doc();
            let count = doc.selection().len();
            Some((1..=count).map(|i| i.to_string()).collect())
        }
        '%' => {
            let doc = ed.active_doc();
            let p = doc
                .path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            Some(vec![p])
        }
        '+' | '*' => None,
        _ => ed.registers.read(reg),
    }
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

    let count = values.len();
    match ed.registers.write(reg, values) {
        Ok(()) => {
            ed.status_message = Some(format!(
                "yanked {count} selection{} to register \"{reg}\"",
                if count == 1 { "" } else { "s" }
            ));
        }
        Err(err) => {
            ed.status_message = Some(err);
        }
    }
}

/// Deletes selected text with or without yanking to active register.
pub fn delete_selection(ed: &mut editor, yank: bool) {
    let reg = ed.selected_register.take().unwrap_or('"');
    if yank && reg != '_' {
        let doc = ed.active_doc();
        let mut values = Vec::new();
        for r in doc.selection().ranges() {
            let from = r.from();
            let to = r.to().max(from + 1).min(doc.text().char_len());
            values.push(doc.text().slice_chars(from..to));
        }
        let _ = ed.registers.write(reg, values);
    }
    delete_selected_text(ed);
}

/// Changes selected text with or without yanking, entering insert mode.
pub fn change_selection(ed: &mut editor, yank: bool) {
    delete_selection(ed, yank);
    ed.mode = ned_config::mode::insert;
}

/// Replaces each selection with a single typed character literal.
pub fn replace_selection_with_char(ed: &mut editor, ch: char) {
    let doc_id = ed.active_doc_id;
    let doc = ed.active_doc_mut();
    let doc_len = doc.text().char_len();
    let mut cs = change_set::new(doc_len);
    let mut last_pos = 0;
    let repl = ch.to_string();

    for r in doc.selection().ranges() {
        let from = r.from();
        let to = r.to().max(from + 1).min(doc_len);
        if from > last_pos {
            cs.retain(from - last_pos);
        }
        if to > from {
            cs.delete(to - from);
        }
        cs.insert(&repl);
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

/// Replaces each selection range with corresponding text from the active register.
pub fn replace_with_yanked(ed: &mut editor) {
    let reg = ed.selected_register.take().unwrap_or('"');
    let values = match read_register(ed, reg) {
        Some(v) if !v.is_empty() => v,
        _ => return,
    };

    let doc_id = ed.active_doc_id;
    let doc = ed.active_doc_mut();
    let doc_len = doc.text().char_len();
    let mut cs = change_set::new(doc_len);
    let mut last_pos = 0;
    let mut new_ranges = Vec::new();
    let mut current_offset: isize = 0;

    let sel_ranges: Vec<_> = doc.selection().ranges().to_vec();
    for (i, r) in sel_ranges.iter().enumerate() {
        let from = r.from();
        let to = r.to().max(from + 1).min(doc_len);
        if from > last_pos {
            cs.retain(from - last_pos);
        }
        if to > from {
            cs.delete(to - from);
        }
        let repl = if values.len() == 1 {
            &values[0]
        } else if i < values.len() {
            &values[i]
        } else {
            values.last().unwrap()
        };
        cs.insert(repl);
        let repl_char_len = repl.chars().count();
        let new_anchor = (from as isize + current_offset) as usize;
        let new_head = new_anchor + repl_char_len;
        new_ranges.push(ned_text::range::new(new_anchor, new_head));
        current_offset += repl_char_len as isize - (to - from) as isize;
        last_pos = to;
    }
    if last_pos < doc_len {
        cs.retain(doc_len - last_pos);
    }

    let new_sel = ned_text::selection::new(new_ranges, doc.selection().primary_index());
    doc.apply(cs, Some(new_sel));
    ed.ensure_active_cursor_visible();
    ed.notify_lsp_did_change(doc_id);
}

/// Pastes text from active register after or before cursor with cardinality and linewise rules.
pub fn paste_register(ed: &mut editor, after: bool) {
    let is_normal_mode = ed.mode == ned_config::mode::normal;
    let reg = ed.selected_register.take().unwrap_or('"');
    let values = match read_register(ed, reg) {
        Some(v) if !v.is_empty() => v,
        _ => return,
    };

    let doc_id = ed.active_doc_id;
    let doc = ed.active_doc_mut();
    let text = doc.text();
    let doc_len = text.char_len();

    let linewise = values.iter().any(|v| v.ends_with('\n') || v.ends_with("\r\n"));
    let sel_ranges: Vec<_> = doc.selection().ranges().to_vec();
    let num_selections = sel_ranges.len();

    struct paste_op {
        target_char: usize,
        content: String,
    }

    let mut ops = Vec::with_capacity(num_selections);

    for (i, r) in sel_ranges.iter().enumerate() {
        let content = if values.len() == 1 {
            values[0].clone()
        } else if i < values.len() {
            values[i].clone()
        } else {
            values.last().unwrap().clone()
        };

        let target_char = if linewise {
            if after {
                let line_byte = text.char_to_byte(r.to().saturating_sub(1));
                let line_idx = text.byte_to_line(line_byte);
                let next_line = (line_idx + 1).min(text.line_count());
                let next_byte = text.line_to_byte(next_line).unwrap_or(text.byte_len());
                text.byte_to_char(next_byte)
            } else {
                let line_byte = text.char_to_byte(r.from());
                let line_idx = text.byte_to_line(line_byte);
                let curr_byte = text.line_to_byte(line_idx).unwrap_or(0);
                text.byte_to_char(curr_byte)
            }
        } else if after {
            r.to().min(doc_len)
        } else {
            r.from().min(doc_len)
        };

        ops.push(paste_op {
            target_char,
            content,
        });
    }

    ops.sort_by_key(|op| op.target_char);

    let mut cs = change_set::new(doc_len);
    let mut last_pos = 0;
    let mut new_ranges = Vec::with_capacity(ops.len());
    let mut current_offset: isize = 0;

    for op in ops {
        if op.target_char > last_pos {
            cs.retain(op.target_char - last_pos);
            last_pos = op.target_char;
        }
        cs.insert(&op.content);
        let content_char_len = op.content.chars().count();
        let new_anchor = (op.target_char as isize + current_offset) as usize;
        let new_head = new_anchor + content_char_len;
        new_ranges.push(ned_text::range::new(new_anchor, new_head));
        current_offset += content_char_len as isize;
    }
    if last_pos < doc_len {
        cs.retain(doc_len - last_pos);
    }

    let new_sel = if is_normal_mode {
        Some(ned_text::selection::new(
            new_ranges,
            doc.selection().primary_index(),
        ))
    } else {
        None
    };

    doc.apply(cs, new_sel);
    ed.ensure_active_cursor_visible();
    ed.notify_lsp_did_change(doc_id);
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
