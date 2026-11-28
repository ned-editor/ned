//! Built-in grammar-independent text objects: words, paragraphs, brackets, and quotes.

use ned_rope::rope;

use crate::chars::{categorize_char, char_category, char_is_whitespace};
use crate::graphemes::next_grapheme_boundary;
use crate::line_endings::char_is_line_ending;
use crate::ranges::range;

/// Text object selection mode: whether to select inside or around the target object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum textobject_mode {
    /// Select content inside delimiters or without surrounding whitespace.
    inside,
    /// Select content including enclosing delimiters or adjacent whitespace.
    around,
}

/// Returns the matching open and close delimiters for a given delimiter character.
#[must_use]
pub fn get_pair(ch: char) -> (char, char) {
    match ch {
        '(' | ')' => ('(', ')'),
        '[' | ']' => ('[', ']'),
        '{' | '}' => ('{', '}'),
        '<' | '>' => ('<', '>'),
        '‘' | '’' => ('‘', '’'),
        '“' | '”' => ('“', '”'),
        '«' | '»' => ('«', '»'),
        '「' | '」' => ('「', '」'),
        '（' | '）' => ('（', '）'),
        '"' => ('"', '"'),
        '\'' => ('\'', '\''),
        '`' => ('`', '`'),
        other => (other, other),
    }
}

/// Checks whether a character at `pos` in `buffer` is preceded by an odd number of backslashes.
#[must_use]
pub fn is_escaped_at(buffer: &rope, pos: usize) -> bool {
    if pos == 0 {
        return false;
    }
    let mut count = 0;
    let mut idx = pos;
    while idx > 0 && count < 16 {
        idx -= 1;
        if buffer.char_at(idx) == Some('\\') {
            count += 1;
        } else {
            break;
        }
    }
    count % 2 == 1
}

/// Computes the word or WORD text object range around the cursor.
#[must_use]
pub fn textobject_word(buffer: &rope, r: range, mode: textobject_mode, long: bool) -> range {
    let len = buffer.char_len();
    if len == 0 {
        return r;
    }
    let pos = r.cursor(buffer).min(len.saturating_sub(1));

    let (word_start, word_end) = if long {
        let ch = match buffer.char_at(pos) {
            Some(c) => c,
            None => return r,
        };
        if char_is_whitespace(ch) || char_is_line_ending(ch) {
            (pos, pos)
        } else {
            let mut start = pos;
            while start > 0 {
                if let Some(c) = buffer.char_at(start - 1) {
                    if !char_is_whitespace(c) && !char_is_line_ending(c) {
                        start -= 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            let mut end = pos + 1;
            while end < len {
                if let Some(c) = buffer.char_at(end) {
                    if !char_is_whitespace(c) && !char_is_line_ending(c) {
                        end += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            (start, end)
        }
    } else {
        let ch = match buffer.char_at(pos) {
            Some(c) => c,
            None => return r,
        };
        let cat = categorize_char(ch);
        if cat == char_category::whitespace || cat == char_category::eol {
            (pos, pos)
        } else {
            let mut start = pos;
            while start > 0 {
                if let Some(c) = buffer.char_at(start - 1) {
                    if categorize_char(c) == cat {
                        start -= 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            let mut end = pos + 1;
            while end < len {
                if let Some(c) = buffer.char_at(end) {
                    if categorize_char(c) == cat {
                        end += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            (start, end)
        }
    };

    if word_start == word_end {
        return range::new(word_start, word_end).with_direction(r.direction());
    }

    match mode {
        textobject_mode::inside => range::new(word_start, word_end).with_direction(r.direction()),
        textobject_mode::around => {
            let mut ws_right = 0;
            while word_end + ws_right < len {
                if let Some(c) = buffer.char_at(word_end + ws_right) {
                    if char_is_whitespace(c) {
                        ws_right += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            if ws_right > 0 {
                range::new(word_start, word_end + ws_right).with_direction(r.direction())
            } else {
                let mut ws_left = 0;
                while word_start > ws_left {
                    if let Some(c) = buffer.char_at(word_start - 1 - ws_left) {
                        if char_is_whitespace(c) {
                            ws_left += 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                range::new(word_start.saturating_sub(ws_left), word_end)
                    .with_direction(r.direction())
            }
        }
    }
}

/// Helper function to check if a line in the buffer contains only whitespace or is empty.
fn is_line_blank(buffer: &rope, line_idx: usize) -> bool {
    let line_str = match buffer.line(line_idx) {
        Some(s) => s,
        None => return true,
    };
    line_str
        .chars()
        .all(|c| char_is_whitespace(c) || char_is_line_ending(c))
}

/// Computes the paragraph text object range around the cursor.
#[must_use]
pub fn textobject_paragraph(buffer: &rope, r: range, mode: textobject_mode, count: usize) -> range {
    let line_count = buffer.line_count();
    if line_count == 0 {
        return r;
    }

    let cursor_line = r.cursor_line(buffer).min(line_count.saturating_sub(1));
    let start_blank = is_line_blank(buffer, cursor_line);

    // Find start of the current paragraph block
    let mut first_line = cursor_line;
    while first_line > 0 {
        let prev_blank = is_line_blank(buffer, first_line - 1);
        if prev_blank == start_blank {
            first_line -= 1;
        } else {
            break;
        }
    }

    // Advance forward through `count` paragraph blocks
    let mut last_line = cursor_line;
    let mut current_is_blank = start_blank;
    let mut paragraphs_done = 0;

    while last_line + 1 < line_count {
        let next_blank = is_line_blank(buffer, last_line + 1);
        if next_blank != current_is_blank {
            if !current_is_blank {
                paragraphs_done += 1;
                if paragraphs_done >= count.max(1) {
                    break;
                }
            }
            current_is_blank = next_blank;
        }
        last_line += 1;
    }

    // For `around` mode, consume trailing blank lines after the non-blank paragraph
    if mode == textobject_mode::around && !start_blank {
        while last_line + 1 < line_count && is_line_blank(buffer, last_line + 1) {
            last_line += 1;
        }
    }

    let start_byte = buffer.line_to_byte(first_line).unwrap_or(0);
    let start_char = buffer.byte_to_char(start_byte);
    let end_byte = buffer
        .line_to_byte(last_line + 1)
        .unwrap_or_else(|| buffer.byte_len());
    let end_char = buffer.byte_to_char(end_byte);

    range::new(start_char, end_char).with_direction(r.direction())
}

/// Maximum character scanning distance for finding matching delimiters (1 MiB).
const MAX_SCAN_CHARS: usize = 1_000_000;

/// Finds the `(open_pos, close_pos)` offsets of the `n`-th pair of `ch` enclosing `r`.
#[must_use]
pub fn find_nth_pairs_pos(
    buffer: &rope,
    ch: char,
    r: range,
    count: usize,
) -> Option<(usize, usize)> {
    let len = buffer.char_len();
    if len < 2 {
        return None;
    }

    let (open, close) = get_pair(ch);
    let count = count.max(1);

    if open == close {
        // Symmetric pair (single quotes, double quotes, backticks)
        let pos = r.cursor(buffer).min(len.saturating_sub(1));
        let byte_pos = buffer.char_to_byte(pos);
        let line_idx = buffer.byte_to_line(byte_pos);
        let line_start_byte = buffer.line_to_byte(line_idx).unwrap_or(0);
        let line_start_char = buffer.byte_to_char(line_start_byte);
        let line_end_byte = buffer
            .line_to_byte(line_idx + 1)
            .unwrap_or_else(|| buffer.byte_len());
        let line_end_char = buffer.byte_to_char(line_end_byte);

        // Scan unescaped occurrences of `open` on the containing line
        let mut indices = Vec::new();
        for idx in line_start_char..line_end_char {
            if is_escaped_at(buffer, idx) {
                continue;
            }
            if buffer.char_at(idx) == Some(open) {
                indices.push(idx);
            }
        }

        if indices.len() < 2 {
            return None;
        }

        // Pairs are formed by (indices[2k], indices[2k + 1])
        let mut candidate_pairs = Vec::new();
        for chunk in indices.chunks_exact(2) {
            candidate_pairs.push((chunk[0], chunk[1]));
        }

        // Find enclosing pair
        let mut enclosing = Vec::new();
        for (o, c) in candidate_pairs {
            if o <= r.from() && c >= r.to().saturating_sub(1) {
                enclosing.push((o, c));
            }
        }

        if enclosing.is_empty() {
            // Also check backward and forward scan if outside strict chunk bounds
            let mut left_delim = None;
            let mut right_delim = None;

            for &idx in indices.iter().rev() {
                if idx <= r.from() {
                    left_delim = Some(idx);
                    break;
                }
            }
            for &idx in &indices {
                if idx >= r.to().saturating_sub(1) && Some(idx) != left_delim {
                    right_delim = Some(idx);
                    break;
                }
            }

            if let (Some(o), Some(c)) = (left_delim, right_delim) {
                if o < c {
                    return Some((o, c));
                }
            }
            return None;
        }

        let idx = (count - 1).min(enclosing.len() - 1);
        Some(enclosing[idx])
    } else {
        // Asymmetric pair (parentheses, brackets, braces, angle brackets)
        let search_start = r.from();
        let min_bound = search_start.saturating_sub(MAX_SCAN_CHARS);

        let mut open_pos = None;
        let mut depth = 0;
        let mut found_count = 0;

        // If cursor is right on the open delimiter, include it
        let mut idx = (search_start + 1).min(len);
        while idx > min_bound {
            idx -= 1;
            if is_escaped_at(buffer, idx) {
                continue;
            }
            if let Some(c) = buffer.char_at(idx) {
                if c == close && idx != search_start {
                    depth += 1;
                } else if c == open {
                    if depth == 0 {
                        found_count += 1;
                        if found_count == count {
                            open_pos = Some(idx);
                            break;
                        }
                    } else {
                        depth -= 1;
                    }
                }
            }
        }

        let o_pos = open_pos?;

        // Scan forward from `o_pos + 1` to find the exact matching closing delimiter
        let max_bound = (o_pos + MAX_SCAN_CHARS).min(len);
        let mut close_depth = 1;
        let mut close_pos = None;

        for f_idx in (o_pos + 1)..max_bound {
            if is_escaped_at(buffer, f_idx) {
                continue;
            }
            if let Some(c) = buffer.char_at(f_idx) {
                if c == open {
                    close_depth += 1;
                } else if c == close {
                    close_depth -= 1;
                    if close_depth == 0 {
                        close_pos = Some(f_idx);
                        break;
                    }
                }
            }
        }

        let c_pos = close_pos?;
        // Verify enclosing range condition
        if o_pos <= r.from() && c_pos >= r.to().saturating_sub(1) {
            Some((o_pos, c_pos))
        } else {
            None
        }
    }
}

/// Finds the closest enclosing delimiter pair among standard brackets and quotes.
#[must_use]
pub fn find_nth_closest_pairs_pos(buffer: &rope, r: range, count: usize) -> Option<(usize, usize)> {
    const CANDIDATES: [char; 7] = ['(', '[', '{', '<', '"', '\'', '`'];
    let mut found_pairs = Vec::new();

    for &ch in &CANDIDATES {
        if let Some((o, c)) = find_nth_pairs_pos(buffer, ch, r, 1) {
            found_pairs.push((o, c));
        }
    }

    if found_pairs.is_empty() {
        return None;
    }

    // Sort by span length (close - open) ascending to pick the tightest enclosing pair
    found_pairs.sort_by_key(|&(o, c)| c.saturating_sub(o));

    let idx = (count.max(1) - 1).min(found_pairs.len() - 1);
    Some(found_pairs[idx])
}

/// Selects inside or around the pair matching delimiter character `ch`.
#[must_use]
pub fn textobject_pair_surround(
    buffer: &rope,
    r: range,
    mode: textobject_mode,
    ch: char,
    count: usize,
) -> Option<range> {
    let (open_pos, close_pos) = find_nth_pairs_pos(buffer, ch, r, count)?;
    build_pair_range(buffer, r, mode, open_pos, close_pos)
}

/// Selects inside or around the closest surrounding delimiter pair (`m`).
#[must_use]
pub fn textobject_pair_surround_closest(
    buffer: &rope,
    r: range,
    mode: textobject_mode,
    count: usize,
) -> Option<range> {
    let (open_pos, close_pos) = find_nth_closest_pairs_pos(buffer, r, count)?;
    build_pair_range(buffer, r, mode, open_pos, close_pos)
}

fn build_pair_range(
    buffer: &rope,
    r: range,
    mode: textobject_mode,
    open_pos: usize,
    close_pos: usize,
) -> Option<range> {
    match mode {
        textobject_mode::inside => {
            let from = next_grapheme_boundary(buffer, open_pos);
            let to = close_pos;
            if from <= to {
                Some(range::new(from, to).with_direction(r.direction()))
            } else {
                Some(range::new(from, from).with_direction(r.direction()))
            }
        }
        textobject_mode::around => {
            let from = open_pos;
            let to = next_grapheme_boundary(buffer, close_pos);
            Some(range::new(from, to).with_direction(r.direction()))
        }
    }
}
