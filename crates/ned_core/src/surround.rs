//! Single-character surround operations: add, delete, and replace enclosing delimiter pairs.

use ned_text::{
    change_set, find_nth_closest_pairs_pos, find_nth_pairs_pos, get_pair, range, selection,
};

use crate::documents::document;

/// Pending surround operation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum surround_op {
    /// Add delimiters around selections (`ms<char>`).
    add,
    /// Delete enclosing delimiters around selections (`md<char>`).
    delete,
    /// Replace enclosing delimiters, awaiting the original delimiter character (`mr<old>`).
    replace_first,
    /// Replace enclosing delimiters, awaiting the replacement delimiter character (`mr<old><new>`).
    replace_second(char),
}

/// Adds delimiter characters around each active selection range.
///
/// # Errors
///
/// Returns `Err` if a surround pair cannot be determined for the delimiter character.
pub fn surround_add(doc: &mut document, ch: char) -> Result<(), &'static str> {
    let (open_str, close_str) = if ch == '\n' || ch == '\r' {
        let le = doc.line_ending().as_str();
        (le.to_string(), le.to_string())
    } else {
        let (o, c) = get_pair(ch);
        let mut os = String::new();
        os.push(o);
        let mut cs = String::new();
        cs.push(c);
        (os, cs)
    };

    let doc_len = doc.text().char_len();
    let mut cs = change_set::new(doc_len);
    let mut last_pos = 0;
    let mut new_ranges = Vec::new();
    let mut offset = 0;
    let open_len = open_str.chars().count();
    let close_len = close_str.chars().count();
    let surround_len = open_len + close_len;

    for r in doc.selection().ranges() {
        let from = r.from();
        let to = r.to();

        if from > last_pos {
            cs.retain(from - last_pos);
        }
        cs.insert(&open_str);

        if to > from {
            cs.retain(to - from);
        }
        cs.insert(&close_str);

        last_pos = to;

        new_ranges.push(
            range::new(offset + from, offset + to + surround_len).with_direction(r.direction()),
        );
        offset += surround_len;
    }

    if last_pos < doc_len {
        cs.retain(doc_len - last_pos);
    }

    let primary_idx = doc.selection().primary_index();
    let new_sel = selection::new(new_ranges, primary_idx);
    doc.apply(&cs, Some(new_sel));
    Ok(())
}

/// Deletes the enclosing delimiter pair around each active selection range.
///
/// # Errors
///
/// Returns `Err("Surround pair not found around all cursors")` if a matching pair is not found
/// for any selection, or `Err("Cursors overlap for surround pair")` if deletion positions overlap.
pub fn surround_delete(doc: &mut document, ch: char, count: usize) -> Result<(), &'static str> {
    let mut del_positions = Vec::new();

    for r in doc.selection().ranges() {
        let pair = if ch == 'm' {
            find_nth_closest_pairs_pos(doc.text(), *r, count)
        } else {
            find_nth_pairs_pos(doc.text(), ch, *r, count)
        };

        match pair {
            Some((open_pos, close_pos)) => {
                if del_positions.contains(&open_pos) || del_positions.contains(&close_pos) {
                    return Err("Cursors overlap for surround pair");
                }
                del_positions.push(open_pos);
                del_positions.push(close_pos);
            }
            None => {
                return Err("Surround pair not found around all cursors");
            }
        }
    }

    del_positions.sort_unstable();

    let doc_len = doc.text().char_len();
    let mut cs = change_set::new(doc_len);
    let mut last_pos = 0;

    for &pos in &del_positions {
        if pos > last_pos {
            cs.retain(pos - last_pos);
        }
        cs.delete(1);
        last_pos = pos + 1;
    }

    if last_pos < doc_len {
        cs.retain(doc_len - last_pos);
    }

    let new_sel = cs.map_selection(doc.selection().clone());
    doc.apply(&cs, Some(new_sel));
    Ok(())
}

/// Replaces the enclosing delimiter pair around each active selection range.
///
/// # Errors
///
/// Returns `Err("Surround pair not found around all cursors")` if a matching pair is not found
/// for any selection, or `Err("Cursors overlap for surround pair")` if replacement positions overlap.
pub fn surround_replace(
    doc: &mut document,
    old_ch: char,
    new_ch: char,
    count: usize,
) -> Result<(), &'static str> {
    let (new_open, new_close) = get_pair(new_ch);
    let mut replacements: Vec<(usize, char)> = Vec::new();

    for r in doc.selection().ranges() {
        let pair = if old_ch == 'm' {
            find_nth_closest_pairs_pos(doc.text(), *r, count)
        } else {
            find_nth_pairs_pos(doc.text(), old_ch, *r, count)
        };

        match pair {
            Some((open_pos, close_pos)) => {
                if replacements
                    .iter()
                    .any(|(p, _)| *p == open_pos || *p == close_pos)
                {
                    return Err("Cursors overlap for surround pair");
                }
                replacements.push((open_pos, new_open));
                replacements.push((close_pos, new_close));
            }
            None => {
                return Err("Surround pair not found around all cursors");
            }
        }
    }

    replacements.sort_by_key(|&(p, _)| p);

    let doc_len = doc.text().char_len();
    let mut cs = change_set::new(doc_len);
    let mut last_pos = 0;

    for &(pos, new_char) in &replacements {
        if pos > last_pos {
            cs.retain(pos - last_pos);
        }
        cs.delete(1);
        let mut s = String::new();
        s.push(new_char);
        cs.insert(&s);
        last_pos = pos + 1;
    }

    if last_pos < doc_len {
        cs.retain(doc_len - last_pos);
    }

    let new_sel = cs.map_selection(doc.selection().clone());
    doc.apply(&cs, Some(new_sel));
    Ok(())
}
