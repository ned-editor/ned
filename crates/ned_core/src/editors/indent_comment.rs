//! Language token-based editing helpers: smart auto-indentation and comment toggling.

use ned_text::{change_set, selection};

use crate::editors::edit_ops::insert_string;
use crate::editors::editor;

/// Empty slice reference used when language indentation configuration is absent.
const EMPTY_INCREASE_DECREASE: &[String] = &[];

/// Inserts a newline with auto-indentation and comment continuation.
pub fn insert_newline(ed: &mut editor) {
    let (
        base_indent,
        new_indent,
        comment_prefix,
        is_empty_comment,
        chars_to_clear,
        is_split_pair,
        cursor_char,
    ) = {
        let doc = ed.active_doc();
        let cur_char = doc.selection().primary().cursor(doc.text());
        let cur_byte = doc.text().char_to_byte(cur_char);
        let line_idx = doc.text().byte_to_line(cur_byte);
        let line_str = doc.text().line(line_idx).unwrap_or_default();
        let line_start_byte = doc.text().line_to_byte(line_idx).unwrap_or(0);
        let col_byte = cur_byte.saturating_sub(line_start_byte);
        let prefix_before = &line_str[..col_byte.min(line_str.len())];
        let suffix_after = &line_str[col_byte.min(line_str.len())..];

        let base_indent: String = line_str
            .chars()
            .take_while(|&c| c == ' ' || c == '\t')
            .collect();

        let tab_width = doc
            .language()
            .and_then(|l| l.indent.as_ref().map(|i| i.tab_width).or(Some(l.tab_width)))
            .unwrap_or(ed.config.editor.tab_width);
        let indent_step = " ".repeat(tab_width);

        let trimmed_before = prefix_before.trim_end();
        let trimmed_after = suffix_after.trim_start();

        let (custom_increase, custom_decrease) =
            match doc.language().and_then(|l| l.indent.as_ref()) {
                Some(ind) => (ind.increase.as_slice(), ind.decrease.as_slice()),
                None => (EMPTY_INCREASE_DECREASE, EMPTY_INCREASE_DECREASE),
            };

        let is_opener = trimmed_before.ends_with('{')
            || trimmed_before.ends_with('(')
            || trimmed_before.ends_with('[')
            || custom_increase
                .iter()
                .any(|tok| trimmed_before.ends_with(tok.as_str()));

        let is_closer_after = trimmed_after.starts_with('}')
            || trimmed_after.starts_with(')')
            || trimmed_after.starts_with(']')
            || custom_decrease
                .iter()
                .any(|tok| trimmed_after.starts_with(tok.as_str()));

        let comment_token = doc
            .language()
            .and_then(|l| l.comment_token.as_deref())
            .unwrap_or("//");
        let trimmed_full = prefix_before.trim();

        let (cp, empty_comment) = if trimmed_full == comment_token
            || trimmed_full == "//"
            || trimmed_full == "#"
            || trimmed_full == "*"
        {
            (None, true)
        } else if trimmed_full.starts_with(comment_token) {
            (Some(format!("{comment_token} ")), false)
        } else if trimmed_full.starts_with("//") {
            (Some("// ".to_string()), false)
        } else if trimmed_full.starts_with('#') {
            (Some("# ".to_string()), false)
        } else if trimmed_full.starts_with("/*") || trimmed_full.starts_with('*') {
            (Some("* ".to_string()), false)
        } else {
            (None, false)
        };

        let clear_chars = if empty_comment {
            prefix_before
                .chars()
                .count()
                .saturating_sub(base_indent.chars().count())
        } else {
            0
        };

        if is_opener && is_closer_after {
            let mut inc = base_indent.clone();
            inc.push_str(&indent_step);
            (
                base_indent.clone(),
                inc,
                cp,
                empty_comment,
                clear_chars,
                true,
                cur_char,
            )
        } else if is_opener {
            let mut inc = base_indent.clone();
            inc.push_str(&indent_step);
            (
                base_indent.clone(),
                inc,
                cp,
                empty_comment,
                clear_chars,
                false,
                cur_char,
            )
        } else if is_closer_after {
            let dec_len = base_indent.len().saturating_sub(tab_width);
            let dec = base_indent[..dec_len].to_string();
            (
                base_indent.clone(),
                dec,
                cp,
                empty_comment,
                clear_chars,
                false,
                cur_char,
            )
        } else {
            (
                base_indent.clone(),
                base_indent,
                cp,
                empty_comment,
                clear_chars,
                false,
                cur_char,
            )
        }
    };

    let le = ed.active_doc().line_ending().as_str();

    if is_empty_comment {
        if chars_to_clear > 0 {
            let doc = ed.active_doc_mut();
            let doc_len = doc.text().char_len();
            let mut cs = change_set::new(doc_len);
            let del_start = cursor_char.saturating_sub(chars_to_clear);
            if del_start > 0 {
                cs.retain(del_start);
            }
            cs.delete(chars_to_clear);
            if doc_len > cursor_char {
                cs.retain(doc_len - cursor_char);
            }
            let new_sel = cs.map_selection(doc.selection().clone());
            doc.apply(cs, Some(new_sel));
        }
        let newline_str = format!("{le}{base_indent}");
        insert_string(ed, &newline_str);
    } else if is_split_pair {
        let text_to_insert = format!("{le}{new_indent}{le}{base_indent}");
        insert_string(ed, &text_to_insert);
        let target_cursor = cursor_char + le.chars().count() + new_indent.chars().count();
        let doc = ed.active_doc_mut();
        doc.set_selection(selection::point(target_cursor));
        ed.ensure_active_cursor_visible();
    } else {
        let mut newline_str = format!("{le}{new_indent}");
        if let Some(cp) = comment_prefix {
            newline_str.push_str(&cp);
        }
        insert_string(ed, &newline_str);
    }
}

/// Decreases indentation if the typed character is a closer token and the current line
/// before the cursor contains solely indentation whitespace.
///
/// Returns `true` if indentation was decreased.
pub fn maybe_decrease_indent_on_closer(ed: &mut editor, ch: char) -> bool {
    let doc = ed.active_doc();
    let (custom_decrease, tab_width) = match doc.language() {
        Some(lang) => {
            let tw = lang
                .indent
                .as_ref()
                .map_or(lang.tab_width, |i| i.tab_width);
            let dec = lang.indent.as_ref().map(|i| i.decrease.as_slice());
            (dec, tw)
        }
        None => (None, ed.config.editor.tab_width),
    };

    let is_closer = ch == '}'
        || ch == ')'
        || ch == ']'
        || custom_decrease.is_some_and(|decs| decs.iter().any(|tok| tok.starts_with(ch)));

    if !is_closer {
        return false;
    }

    let cursor_char = doc.selection().primary().cursor(doc.text());
    let cursor_byte = doc.text().char_to_byte(cursor_char);
    let line_idx = doc.text().byte_to_line(cursor_byte);
    let line_str = doc.text().line(line_idx).unwrap_or_default();
    let line_start_byte = doc.text().line_to_byte(line_idx).unwrap_or(0);
    let col_byte = cursor_byte.saturating_sub(line_start_byte);
    let prefix_before = &line_str[..col_byte.min(line_str.len())];

    if prefix_before.is_empty() || !prefix_before.chars().all(|c| c == ' ' || c == '\t') {
        return false;
    }

    let spaces_to_remove = if prefix_before.ends_with('\t') {
        1
    } else {
        let trailing_spaces = prefix_before
            .chars()
            .rev()
            .take_while(|&c| c == ' ')
            .count();
        trailing_spaces.min(tab_width).max(1)
    };

    let doc = ed.active_doc_mut();
    let doc_len = doc.text().char_len();
    let mut cs = change_set::new(doc_len);
    let del_start = cursor_char.saturating_sub(spaces_to_remove);
    if del_start > 0 {
        cs.retain(del_start);
    }
    cs.delete(spaces_to_remove);
    if doc_len > cursor_char {
        cs.retain(doc_len - cursor_char);
    }

    let new_sel = cs.map_selection(doc.selection().clone());
    doc.apply(cs, Some(new_sel));
    ed.ensure_active_cursor_visible();
    true
}

/// Toggles line comments across lines intersecting active selections.
pub fn toggle_line_comment(ed: &mut editor) {
    let doc = ed.active_doc_mut();
    let buffer = doc.text();
    let total_lines = buffer.line_count();
    if total_lines == 0 {
        return;
    }

    let comment_token = doc
        .language()
        .and_then(|l| l.comment_token.as_deref())
        .unwrap_or("//")
        .to_string();

    let mut lines: Vec<usize> = Vec::new();
    for r in doc.selection().ranges() {
        let s_byte = buffer.char_to_byte(r.from());
        let e_byte = buffer.char_to_byte(r.to());
        let start_line = buffer.byte_to_line(s_byte);
        let mut end_line = buffer.byte_to_line(e_byte);
        if r.to() > r.from()
            && buffer.line_to_byte(end_line) == Some(e_byte)
            && end_line > start_line
        {
            end_line -= 1;
        }
        for line in start_line..=end_line {
            if line < total_lines && !lines.contains(&line) {
                lines.push(line);
            }
        }
    }
    lines.sort_unstable();
    if lines.is_empty() {
        return;
    }

    let non_empty_lines: Vec<usize> = lines
        .iter()
        .copied()
        .filter(|&l| {
            let s = buffer.line(l).unwrap_or_default();
            !s.trim().is_empty()
        })
        .collect();

    let all_commented = if non_empty_lines.is_empty() {
        lines.iter().all(|&l| {
            let s = buffer.line(l).unwrap_or_default();
            s.trim_start().starts_with(&comment_token)
        })
    } else {
        non_empty_lines.iter().all(|&l| {
            let s = buffer.line(l).unwrap_or_default();
            s.trim_start().starts_with(&comment_token)
        })
    };

    let doc_len = buffer.char_len();
    let mut cs = change_set::new(doc_len);
    let mut last_pos = 0;

    if all_commented {
        for line in lines {
            let line_str = buffer.line(line).unwrap_or_default();
            let l_start_byte = buffer.line_to_byte(line).unwrap_or(0);
            let l_start_char = buffer.byte_to_char(l_start_byte);

            let indent_len = line_str
                .chars()
                .take_while(|&c| c == ' ' || c == '\t')
                .count();
            let indent_bytes: usize = line_str
                .chars()
                .take(indent_len)
                .map(char::len_utf8)
                .sum();
            let remainder = &line_str[indent_bytes..];

            if remainder.starts_with(&comment_token) {
                let tok_chars = comment_token.chars().count();
                let del_start = l_start_char + indent_len;
                let has_trailing_space = remainder[comment_token.len()..].starts_with(' ');
                let del_len = if has_trailing_space {
                    tok_chars + 1
                } else {
                    tok_chars
                };

                if del_start > last_pos {
                    cs.retain(del_start - last_pos);
                }
                cs.delete(del_len);
                last_pos = del_start + del_len;
            }
        }
    } else {
        let comment_prefix = format!("{comment_token} ");
        for line in lines {
            let line_str = buffer.line(line).unwrap_or_default();
            let l_start_byte = buffer.line_to_byte(line).unwrap_or(0);
            let l_start_char = buffer.byte_to_char(l_start_byte);

            let indent_len = line_str
                .chars()
                .take_while(|&c| c == ' ' || c == '\t')
                .count();
            let insert_pos = l_start_char + indent_len;

            if insert_pos > last_pos {
                cs.retain(insert_pos - last_pos);
            }
            cs.insert(&comment_prefix);
            last_pos = insert_pos;
        }
    }

    if last_pos < doc_len {
        cs.retain(doc_len - last_pos);
    }

    let new_sel = cs.map_selection(doc.selection().clone());
    doc.apply(cs, Some(new_sel));
    ed.ensure_active_cursor_visible();
}

/// Toggles block comments enclosing active selections.
pub fn toggle_block_comment(ed: &mut editor) {
    let doc = ed.active_doc_mut();
    let buffer = doc.text();
    let doc_len = buffer.char_len();
    if doc_len == 0 {
        return;
    }

    let (open_token, close_token) =
        match doc.language().and_then(|l| l.block_comment_tokens.as_ref()) {
            Some(tokens) if tokens.len() == 2 => (tokens[0].clone(), tokens[1].clone()),
            _ => ("/*".to_string(), "*/".to_string()),
        };

    let mut ranges: Vec<(usize, usize)> = doc
        .selection()
        .ranges()
        .iter()
        .map(|r| {
            if r.from() == r.to() {
                let s_byte = buffer.char_to_byte(r.from());
                let line = buffer.byte_to_line(s_byte);
                let l_start_byte = buffer.line_to_byte(line).unwrap_or(0);
                let l_start_char = buffer.byte_to_char(l_start_byte);
                let line_str = buffer.line(line).unwrap_or_default();
                let trimmed_chars = line_str.trim_end_matches(['\r', '\n']).chars().count();
                (l_start_char, l_start_char + trimmed_chars)
            } else {
                (r.from(), r.to())
            }
        })
        .collect();

    ranges.sort_unstable_by_key(|&(from, _)| from);

    let all_commented = ranges.iter().all(|&(from, to)| {
        if to <= from {
            return false;
        }
        let text = buffer.slice_chars(from..to);
        let trimmed = text.trim();
        trimmed.starts_with(&open_token)
            && trimmed.ends_with(&close_token)
            && trimmed.len() >= open_token.len() + close_token.len()
    });

    let mut cs = change_set::new(doc_len);
    let mut last_pos = 0;

    if all_commented {
        for &(from, to) in &ranges {
            let text = buffer.slice_chars(from..to);
            if let Some(open_rel_byte) = text.find(&open_token) {
                let open_rel_char = text[..open_rel_byte].chars().count();
                let open_tok_chars = open_token.chars().count();
                let after_open_byte = open_rel_byte + open_token.len();
                let open_del_extra = usize::from(text[after_open_byte..].starts_with(' '));
                let open_del_chars = open_tok_chars + open_del_extra;

                let open_start = from + open_rel_char;
                if open_start > last_pos {
                    cs.retain(open_start - last_pos);
                }
                cs.delete(open_del_chars);
                last_pos = open_start + open_del_chars;

                if let Some(close_rel_byte) = text.rfind(&close_token) {
                    let before_close_extra = usize::from(text[..close_rel_byte].ends_with(' '));
                    let close_rel_char =
                        text[..close_rel_byte].chars().count() - before_close_extra;
                    let close_del_chars = close_token.chars().count() + before_close_extra;
                    let close_start = from + close_rel_char;

                    if close_start > last_pos {
                        cs.retain(close_start - last_pos);
                    }
                    cs.delete(close_del_chars);
                    last_pos = close_start + close_del_chars;
                }
            }
        }
    } else {
        let open_str = format!("{open_token} ");
        let close_str = format!(" {close_token}");
        for &(from, to) in &ranges {
            if from > last_pos {
                cs.retain(from - last_pos);
            }
            cs.insert(&open_str);
            if to > from {
                cs.retain(to - from);
            }
            cs.insert(&close_str);
            last_pos = to;
        }
    }

    if last_pos < doc_len {
        cs.retain(doc_len - last_pos);
    }

    let new_sel = cs.map_selection(doc.selection().clone());
    doc.apply(cs, Some(new_sel));
    ed.ensure_active_cursor_visible();
}
