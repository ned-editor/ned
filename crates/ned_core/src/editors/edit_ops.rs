//! Document editing operations: insert, delete, change, yank, paste, casing, and indentation.

use ned_config::action;
use ned_regex::regex;
use ned_text::{change_set, prev_grapheme_boundary, range, selection};

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
        '+' => crate::clipboard::read(crate::clipboard::clipboard_target::clipboard).ok(),
        '*' => crate::clipboard::read(crate::clipboard::clipboard_target::primary).ok(),
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
        if let Err(err) = ed.registers.write(reg, values) {
            ed.status_message = Some(err);
        }
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
        _ => {
            if reg == '+' {
                if let Err(e) = crate::clipboard::read(crate::clipboard::clipboard_target::clipboard) {
                    ed.status_message = Some(e);
                }
            } else if reg == '*' {
                if let Err(e) = crate::clipboard::read(crate::clipboard::clipboard_target::primary) {
                    ed.status_message = Some(e);
                }
            }
            return;
        }
    };

    let doc_id = ed.active_doc_id;
    let doc = ed.active_doc_mut();
    let text = doc.text();
    let doc_len = text.char_len();

    let linewise = values
        .iter()
        .any(|v| v.ends_with('\n') || v.ends_with("\r\n"));
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

/// Maximum number of selections allowed per document.
const MAX_SELECTIONS: usize = 10_000;

/// Rotates the textual contents of the selection forward or backward.
pub fn rotate_selection_contents(ed: &mut editor, forward: bool, count: usize) {
    let doc_id = ed.active_doc_id;
    let doc = ed.active_doc_mut();
    let sel = doc.selection();
    if sel.len() < 2 {
        return;
    }
    let mut fragments: Vec<String> = sel
        .ranges()
        .iter()
        .map(|r| doc.text().slice_chars(r.from()..r.to()))
        .collect();
    let rotate_by = count % fragments.len();
    if rotate_by == 0 {
        return;
    }
    if forward {
        fragments.rotate_right(rotate_by);
    } else {
        fragments.rotate_left(rotate_by);
    }
    let doc_len = doc.text().char_len();
    let mut cs = change_set::new(doc_len);
    let mut last_pos = 0;
    let mut current_offset: isize = 0;
    let mut new_ranges = Vec::with_capacity(sel.len());

    for (r, frag) in sel.ranges().iter().zip(fragments.iter()) {
        let from = r.from();
        let to = r.to();
        let old_len = to - from;
        let frag_len = frag.chars().count();

        let new_from = ((from as isize) + current_offset) as usize;
        let new_to = new_from + frag_len;
        let new_r = if r.anchor > r.head {
            range::new(new_to, new_from)
        } else {
            range::new(new_from, new_to)
        };
        new_ranges.push(new_r);
        current_offset += (frag_len as isize) - (old_len as isize);

        if from > last_pos {
            cs.retain(from - last_pos);
        }
        if to > from {
            cs.delete(to - from);
        }
        cs.insert(frag);
        last_pos = to;
    }
    if last_pos < doc_len {
        cs.retain(doc_len - last_pos);
    }
    let new_primary = if forward {
        (sel.primary_index() + rotate_by) % sel.len()
    } else {
        (sel.primary_index() + sel.len() - rotate_by) % sel.len()
    };
    let new_sel = selection::new(new_ranges, new_primary);
    doc.apply(cs, Some(new_sel));
    ed.ensure_active_cursor_visible();
    ed.notify_lsp_did_change(doc_id);
}

/// Aligns single-line selections in columns by inserting space padding.
pub fn align_selections(ed: &mut editor) {
    let doc = ed.active_doc();
    let buffer = doc.text();
    let sel = doc.selection();

    // Check that every range is on a single line
    for r in sel.ranges() {
        let s_byte = buffer.char_to_byte(r.from());
        let e_byte = buffer.char_to_byte(r.to());
        if buffer.byte_to_line(s_byte) != buffer.byte_to_line(e_byte) {
            ed.status_message = Some("align cannot work with multi line selections".to_string());
            return;
        }
    }

    struct range_coord {
        from: usize,
        line: usize,
        col: usize,
    }

    let mut coords = Vec::with_capacity(sel.len());
    let mut col_max_widths: Vec<usize> = Vec::new();
    let mut prev_line = usize::MAX;
    let mut col_idx = 0;

    for r in sel.ranges() {
        let s_byte = buffer.char_to_byte(r.from());
        let line = buffer.byte_to_line(s_byte);
        let l_start_byte = buffer.line_to_byte(line).unwrap_or(0);
        let l_start_char = buffer.byte_to_char(l_start_byte);
        let col = r.from().saturating_sub(l_start_char);

        if line != prev_line {
            col_idx = 0;
            prev_line = line;
        }

        if col_idx < col_max_widths.len() {
            col_max_widths[col_idx] = col_max_widths[col_idx].max(col);
        } else {
            col_max_widths.push(col);
        }

        coords.push(range_coord {
            from: r.from(),
            line,
            col,
        });
        col_idx += 1;
    }

    let mut insertions: Vec<(usize, usize)> = Vec::new();
    prev_line = usize::MAX;
    col_idx = 0;

    for c in &coords {
        if c.line != prev_line {
            col_idx = 0;
            prev_line = c.line;
        }
        let target_col = col_max_widths[col_idx];
        if target_col > c.col {
            let padding = target_col - c.col;
            insertions.push((c.from, padding));
        }
        col_idx += 1;
    }

    if insertions.is_empty() {
        return;
    }

    insertions.sort_unstable_by_key(|&(pos, _)| pos);

    let doc_id = ed.active_doc_id;
    let doc = ed.active_doc_mut();
    let doc_len = doc.text().char_len();
    let mut cs = change_set::new(doc_len);
    let mut last_pos = 0;

    for (pos, pad_len) in insertions {
        if pos > last_pos {
            cs.retain(pos - last_pos);
        }
        let pad_str = " ".repeat(pad_len);
        cs.insert(&pad_str);
        last_pos = pos;
    }
    if last_pos < doc_len {
        cs.retain(doc_len - last_pos);
    }

    let new_sel = cs.map_selection(doc.selection().clone());
    doc.apply(cs, Some(new_sel));
    ed.ensure_active_cursor_visible();
    ed.notify_lsp_did_change(doc_id);
}

/// Joins lines spanning each selection, optionally inserting a single separating space.
pub fn join_selections(ed: &mut editor, with_space: bool) {
    let doc = ed.active_doc();
    let buffer = doc.text();
    let sel = doc.selection();
    let total_lines = buffer.line_count();
    let mut del_ranges: Vec<(usize, usize, &'static str)> = Vec::new();

    for r in sel.ranges() {
        let s_byte = buffer.char_to_byte(r.from());
        let e_byte = buffer.char_to_byte(r.to());
        let start_line = buffer.byte_to_line(s_byte);
        let mut end_line = buffer.byte_to_line(e_byte);
        if start_line == end_line {
            end_line += 1;
        }
        let end_line = end_line.min(total_lines.saturating_sub(1));

        for line_idx in start_line..end_line {
            let line_str = buffer.line(line_idx).unwrap_or_default();
            let l_start_byte = buffer.line_to_byte(line_idx).unwrap_or(0);
            let l_start_char = buffer.byte_to_char(l_start_byte);
            let trimmed_len = line_str.trim_end_matches(['\r', '\n']).chars().count();
            let newline_start_char = l_start_char + trimmed_len;

            let next_line_idx = line_idx + 1;
            if next_line_idx >= total_lines {
                continue;
            }
            let next_start_byte = buffer.line_to_byte(next_line_idx).unwrap_or(0);
            let next_start_char = buffer.byte_to_char(next_start_byte);
            let next_line_str = buffer.line(next_line_idx).unwrap_or_default();

            let whitespace_count = next_line_str
                .chars()
                .take_while(|&c| c == ' ' || c == '\t')
                .count();
            let next_content_char = next_start_char + whitespace_count;

            let sep = if with_space
                && trimmed_len > 0
                && whitespace_count < next_line_str.trim_end_matches(['\r', '\n']).chars().count()
            {
                " "
            } else {
                ""
            };
            del_ranges.push((newline_start_char, next_content_char, sep));
        }
    }

    if del_ranges.is_empty() {
        return;
    }

    del_ranges.sort_unstable_by_key(|&(from, _, _)| from);
    del_ranges.dedup_by_key(|r| r.0);

    let doc_id = ed.active_doc_id;
    let doc = ed.active_doc_mut();
    let doc_len = doc.text().char_len();
    let mut cs = change_set::new(doc_len);
    let mut last_pos = 0;

    for (from, to, sep) in del_ranges {
        if from > last_pos {
            cs.retain(from - last_pos);
        }
        if to > from {
            cs.delete(to - from);
        }
        if !sep.is_empty() {
            cs.insert(sep);
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

/// Copies current selection ranges to adjacent line above or below.
pub fn copy_selection_on_line(ed: &mut editor, forward: bool, count: usize) {
    let doc = ed.active_doc_mut();
    let buffer = doc.text();
    let sel = doc.selection().clone();
    let total_lines = buffer.line_count();
    let mut new_ranges = sel.ranges().to_vec();

    for r in sel.ranges() {
        let cursor_char = r.head;
        let cursor_byte = buffer.char_to_byte(cursor_char);
        let current_line = buffer.byte_to_line(cursor_byte);
        let l_start_byte = buffer.line_to_byte(current_line).unwrap_or(0);
        let l_start_char = buffer.byte_to_char(l_start_byte);
        let col = cursor_char.saturating_sub(l_start_char);

        let anchor_char = r.anchor;
        let anchor_col = anchor_char.saturating_sub(l_start_char);

        for step in 1..=count {
            let target_line = if forward {
                current_line.saturating_add(step)
            } else if current_line >= step {
                current_line - step
            } else {
                break;
            };

            if target_line >= total_lines {
                break;
            }

            let t_start_byte = match buffer.line_to_byte(target_line) {
                Some(b) => b,
                None => break,
            };
            let t_start_char = buffer.byte_to_char(t_start_byte);
            let t_line_str = buffer.line(target_line).unwrap_or_default();
            let t_line_chars = t_line_str.trim_end_matches(['\r', '\n']).chars().count();

            if col <= t_line_chars && anchor_col <= t_line_chars {
                let new_head = t_start_char + col;
                let new_anchor = t_start_char + anchor_col;
                new_ranges.push(range::new(new_anchor, new_head));
                if new_ranges.len() >= MAX_SELECTIONS {
                    break;
                }
            }
        }
        if new_ranges.len() >= MAX_SELECTIONS {
            break;
        }
    }

    doc.set_selection(selection::new(new_ranges, sel.primary_index()));
    ed.ensure_active_cursor_visible();
}

/// Shrinks selection to full line bounds.
pub fn shrink_to_line_bounds(ed: &mut editor) {
    let doc = ed.active_doc_mut();
    let buffer = doc.text();
    let total_chars = buffer.char_len();
    let sel = doc.selection().clone();
    let new_ranges: Vec<range> = sel
        .ranges()
        .iter()
        .map(|r| {
            let start_byte = buffer.char_to_byte(r.from());
            let end_byte = buffer.char_to_byte(r.to());
            let start_line = buffer.byte_to_line(start_byte);
            let end_line = buffer.byte_to_line(end_byte);
            if start_line == end_line {
                return *r;
            }
            let line_start_byte = buffer.line_to_byte(start_line).unwrap_or(0);
            let line_start_char = buffer.byte_to_char(line_start_byte);
            let mut new_from = r.from();
            if r.from() != line_start_char {
                let next_line_byte = buffer
                    .line_to_byte((start_line + 1).min(buffer.line_count()))
                    .unwrap_or(line_start_byte);
                new_from = buffer.byte_to_char(next_line_byte).min(total_chars);
            }
            let end_line_byte = buffer.line_to_byte(end_line).unwrap_or(0);
            let end_line_char = buffer.byte_to_char(end_line_byte);
            let mut new_to = r.to();
            if r.to() != end_line_char {
                new_to = end_line_char.min(total_chars);
            }
            if new_from > new_to {
                new_to = new_from;
            }
            if r.anchor > r.head {
                range::new(new_to, new_from)
            } else {
                range::new(new_from, new_to)
            }
        })
        .collect();
    doc.set_selection(selection::new(new_ranges, sel.primary_index()));
    ed.ensure_active_cursor_visible();
}

/// Selects all sub-matches of `pattern` within existing selection ranges (`s`).
pub fn select_on_matches(ed: &mut editor, pattern: &str) {
    let re = match regex::smart_case(pattern) {
        Ok(r) => r,
        Err(e) => {
            ed.status_message = Some(format!("regex error: {e}"));
            return;
        }
    };
    let _ = ed.registers.write('/', vec![pattern.to_string()]);

    let doc = ed.active_doc_mut();
    let buffer = doc.text();
    let sel = doc.selection();
    let mut new_ranges = Vec::new();

    for r in sel.ranges() {
        let from = r.from();
        let to = r.to();
        if from >= to {
            continue;
        }
        let slice_str = buffer.slice_chars(from..to);
        for m in re.find_iter(&slice_str) {
            let m_start_char = slice_str[..m.start].chars().count();
            let m_end_char = slice_str[..m.end].chars().count();
            let match_from = from + m_start_char;
            let match_to = from + m_end_char;
            if match_from == match_to && match_from == to {
                continue;
            }
            let new_r = if r.anchor > r.head {
                range::new(match_to, match_from)
            } else {
                range::new(match_from, match_to)
            };
            new_ranges.push(new_r);
            if new_ranges.len() >= MAX_SELECTIONS {
                break;
            }
        }
        if new_ranges.len() >= MAX_SELECTIONS {
            break;
        }
    }

    if new_ranges.is_empty() {
        ed.status_message = Some("nothing selected".to_string());
    } else {
        let primary = sel.primary();
        let idx = new_ranges
            .iter()
            .position(|r| r.overlaps(&primary))
            .unwrap_or(0);
        doc.set_selection(selection::new(new_ranges, idx));
        ed.ensure_active_cursor_visible();
    }
}

/// Splits selection ranges on sub-matches of `pattern` (`S`).
pub fn split_on_matches(ed: &mut editor, pattern: &str) {
    let re = match regex::smart_case(pattern) {
        Ok(r) => r,
        Err(e) => {
            ed.status_message = Some(format!("regex error: {e}"));
            return;
        }
    };
    let _ = ed.registers.write('/', vec![pattern.to_string()]);

    let doc = ed.active_doc_mut();
    let buffer = doc.text();
    let sel = doc.selection();
    let mut new_ranges = Vec::new();

    for r in sel.ranges() {
        let from = r.from();
        let to = r.to();
        if from >= to {
            new_ranges.push(*r);
            continue;
        }
        let slice_str = buffer.slice_chars(from..to);
        let mut curr_char = from;
        for m in re.find_iter(&slice_str) {
            let m_start_char = slice_str[..m.start].chars().count();
            let m_end_char = slice_str[..m.end].chars().count();
            let split_start = curr_char;
            let split_end = from + m_start_char;
            if split_start < split_end {
                let new_r = if r.anchor > r.head {
                    range::new(split_end, split_start)
                } else {
                    range::new(split_start, split_end)
                };
                new_ranges.push(new_r);
            }
            curr_char = from + m_end_char;
            if new_ranges.len() >= MAX_SELECTIONS {
                break;
            }
        }
        if curr_char < to && new_ranges.len() < MAX_SELECTIONS {
            let new_r = if r.anchor > r.head {
                range::new(to, curr_char)
            } else {
                range::new(curr_char, to)
            };
            new_ranges.push(new_r);
        }
        if new_ranges.len() >= MAX_SELECTIONS {
            break;
        }
    }

    if new_ranges.is_empty() {
        doc.set_selection(selection::single(sel.primary().anchor, sel.primary().head));
    } else {
        doc.set_selection(selection::new(new_ranges, 0));
        ed.ensure_active_cursor_visible();
    }
}

/// Filters selection ranges matching `pattern` (`K` to keep, `Alt-K` to remove).
pub fn keep_or_remove_matches(ed: &mut editor, pattern: &str, remove: bool) {
    let re = match regex::smart_case(pattern) {
        Ok(r) => r,
        Err(e) => {
            ed.status_message = Some(format!("regex error: {e}"));
            return;
        }
    };
    let _ = ed.registers.write('/', vec![pattern.to_string()]);

    let doc = ed.active_doc_mut();
    let buffer = doc.text();
    let sel = doc.selection();
    let mut new_ranges = Vec::new();

    for r in sel.ranges() {
        let from = r.from();
        let to = r.to();
        let slice_str = buffer.slice_chars(from..to);
        let is_m = re.is_match(&slice_str);
        if is_m ^ remove {
            new_ranges.push(*r);
        }
    }

    if new_ranges.is_empty() {
        ed.status_message = Some("no selections remaining".to_string());
    } else {
        let primary = sel.primary();
        let idx = new_ranges
            .iter()
            .position(|r| r.overlaps(&primary))
            .unwrap_or(0);
        doc.set_selection(selection::new(new_ranges, idx));
        ed.ensure_active_cursor_visible();
    }
}
