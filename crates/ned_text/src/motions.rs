//! Pure motion transformation functions over rope text buffers and selection ranges.

use ned_rope::rope;

use crate::chars::{char_is_whitespace, char_is_word};
use crate::graphemes::{
    ensure_grapheme_boundary_prev, next_grapheme_boundary, nth_next_grapheme_boundary,
    nth_prev_grapheme_boundary, prev_grapheme_boundary,
};
use crate::line_endings::char_is_line_ending;
use crate::ranges::{direction, range};
use crate::selections::selection;

/// Movement behavior: whether to move the cursor point or extend the active selection.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum movement {
    /// Collapses selection to a cursor point at the new destination.
    move_,
    /// Keeps the anchor fixed and extends the head to the new destination.
    extend,
}

/// Moves the cursor horizontally by `count` grapheme clusters.
#[must_use]
pub fn move_horizontally(
    buffer: &rope,
    r: range,
    dir: direction,
    count: usize,
    behavior: movement,
) -> range {
    let pos = r.cursor(buffer);
    let new_pos = match dir {
        direction::forward => nth_next_grapheme_boundary(buffer, pos, count),
        direction::backward => nth_prev_grapheme_boundary(buffer, pos, count),
    };
    r.put_cursor(buffer, new_pos, behavior == movement::extend)
}

/// Moves the cursor vertically by `count` lines, maintaining sticky visual column.
#[must_use]
pub fn move_vertically(
    buffer: &rope,
    r: range,
    dir: direction,
    count: usize,
    behavior: movement,
) -> range {
    if buffer.is_empty() {
        return r;
    }

    let pos = r.cursor(buffer);
    let line_idx = buffer.byte_to_line(buffer.char_to_byte(pos));
    let line_start_byte = buffer.line_to_byte(line_idx).unwrap_or(0);
    let line_start_char = buffer.byte_to_char(line_start_byte);
    let current_col = (pos.saturating_sub(line_start_char)) as u32;

    let sticky_col = r.old_visual_position.map_or(current_col, |(_, col)| col);

    let max_line = buffer.line_count().saturating_sub(1);
    let target_line = match dir {
        direction::forward => (line_idx + count).min(max_line),
        direction::backward => line_idx.saturating_sub(count),
    };

    let target_start_byte = buffer.line_to_byte(target_line).unwrap_or(0);
    let target_start_char = buffer.byte_to_char(target_start_byte);
    let target_end_byte = buffer
        .line_to_byte(target_line + 1)
        .unwrap_or_else(|| buffer.byte_len());
    let mut target_end_char = buffer.byte_to_char(target_end_byte);

    // Strip trailing line ending from target line length so cursor stays on content.
    while target_end_char > target_start_char {
        if let Some(ch) = buffer.char_at(target_end_char - 1) {
            if char_is_line_ending(ch) {
                target_end_char -= 1;
                continue;
            }
        }
        break;
    }

    let target_len = target_end_char.saturating_sub(target_start_char);
    let target_char_offset = (sticky_col as usize).min(target_len);
    let raw_target_pos = target_start_char + target_char_offset;
    let aligned_target_pos = ensure_grapheme_boundary_prev(buffer, raw_target_pos);

    let mut new_range = r.put_cursor(buffer, aligned_target_pos, behavior == movement::extend);
    new_range.old_visual_position = Some((0, sticky_col));
    new_range
}

/// Moves the cursor forward to the start of the next word.
#[must_use]
pub fn move_word_forward(buffer: &rope, r: range, count: usize, behavior: movement) -> range {
    let mut pos = r.cursor(buffer);
    let len = buffer.char_len();

    for _ in 0..count {
        if pos >= len {
            break;
        }

        let start_ch = buffer.char_at(pos).unwrap_or(' ');
        if char_is_word(start_ch) {
            while pos < len {
                if let Some(ch) = buffer.char_at(pos) {
                    if !char_is_word(ch) {
                        break;
                    }
                }
                pos = next_grapheme_boundary(buffer, pos);
            }
        } else if !char_is_whitespace(start_ch) && !char_is_line_ending(start_ch) {
            while pos < len {
                if let Some(ch) = buffer.char_at(pos) {
                    if char_is_word(ch) || char_is_whitespace(ch) || char_is_line_ending(ch) {
                        break;
                    }
                }
                pos = next_grapheme_boundary(buffer, pos);
            }
        }

        // Skip subsequent whitespace
        while pos < len {
            if let Some(ch) = buffer.char_at(pos) {
                if !char_is_whitespace(ch) && !char_is_line_ending(ch) {
                    break;
                }
            }
            pos = next_grapheme_boundary(buffer, pos);
        }
    }

    r.put_cursor(buffer, pos, behavior == movement::extend)
}

/// Moves the cursor backward to the start of the previous word.
#[must_use]
pub fn move_word_backward(buffer: &rope, r: range, count: usize, behavior: movement) -> range {
    let mut pos = r.cursor(buffer);

    for _ in 0..count {
        if pos == 0 {
            break;
        }

        // Skip whitespace backward
        while pos > 0 {
            let prev = prev_grapheme_boundary(buffer, pos);
            if let Some(ch) = buffer.char_at(prev) {
                if !char_is_whitespace(ch) && !char_is_line_ending(ch) {
                    break;
                }
            }
            pos = prev;
        }

        if pos == 0 {
            break;
        }

        let char_before = prev_grapheme_boundary(buffer, pos);
        let ch = buffer.char_at(char_before).unwrap_or(' ');
        if char_is_word(ch) {
            while pos > 0 {
                let prev = prev_grapheme_boundary(buffer, pos);
                if let Some(c) = buffer.char_at(prev) {
                    if !char_is_word(c) {
                        break;
                    }
                }
                pos = prev;
            }
        } else {
            while pos > 0 {
                let prev = prev_grapheme_boundary(buffer, pos);
                if let Some(c) = buffer.char_at(prev) {
                    if char_is_word(c) || char_is_whitespace(c) || char_is_line_ending(c) {
                        break;
                    }
                }
                pos = prev;
            }
        }
    }

    r.put_cursor(buffer, pos, behavior == movement::extend)
}

/// Moves the cursor to the end of the current or next word.
#[must_use]
pub fn move_word_end(buffer: &rope, r: range, count: usize, behavior: movement) -> range {
    let mut pos = r.cursor(buffer);
    let len = buffer.char_len();

    for _ in 0..count {
        pos = next_grapheme_boundary(buffer, pos);
        // Skip whitespace
        while pos < len {
            if let Some(ch) = buffer.char_at(pos) {
                if !char_is_whitespace(ch) && !char_is_line_ending(ch) {
                    break;
                }
            }
            pos = next_grapheme_boundary(buffer, pos);
        }

        if pos >= len {
            break;
        }

        let ch = buffer.char_at(pos).unwrap_or(' ');
        if char_is_word(ch) {
            while pos < len {
                let next = next_grapheme_boundary(buffer, pos);
                if let Some(c) = buffer.char_at(next) {
                    if !char_is_word(c) {
                        break;
                    }
                } else {
                    break;
                }
                pos = next;
            }
        } else {
            while pos < len {
                let next = next_grapheme_boundary(buffer, pos);
                if let Some(c) = buffer.char_at(next) {
                    if char_is_word(c) || char_is_whitespace(c) || char_is_line_ending(c) {
                        break;
                    }
                } else {
                    break;
                }
                pos = next;
            }
        }
    }

    r.put_cursor(buffer, pos, behavior == movement::extend)
}

/// Moves the cursor to the start of the current logical line.
#[must_use]
pub fn move_line_start(buffer: &rope, r: range, behavior: movement) -> range {
    let pos = r.cursor(buffer);
    let line_idx = buffer.byte_to_line(buffer.char_to_byte(pos));
    let line_start_byte = buffer.line_to_byte(line_idx).unwrap_or(0);
    let line_start_char = buffer.byte_to_char(line_start_byte);
    r.put_cursor(buffer, line_start_char, behavior == movement::extend)
}

/// Moves the cursor to the end of the current logical line (before line break).
#[must_use]
pub fn move_line_end(buffer: &rope, r: range, behavior: movement) -> range {
    let pos = r.cursor(buffer);
    let line_idx = buffer.byte_to_line(buffer.char_to_byte(pos));
    let next_line_byte = buffer
        .line_to_byte(line_idx + 1)
        .unwrap_or_else(|| buffer.byte_len());
    let mut line_end_char = buffer.byte_to_char(next_line_byte);

    while line_end_char > 0 {
        if let Some(ch) = buffer.char_at(line_end_char - 1) {
            if char_is_line_ending(ch) {
                line_end_char -= 1;
                continue;
            }
        }
        break;
    }

    r.put_cursor(buffer, line_end_char, behavior == movement::extend)
}

/// Moves the cursor to the first non-whitespace character of the current line.
#[must_use]
pub fn move_first_non_whitespace(buffer: &rope, r: range, behavior: movement) -> range {
    let pos = r.cursor(buffer);
    let line_idx = buffer.byte_to_line(buffer.char_to_byte(pos));
    let line_start_byte = buffer.line_to_byte(line_idx).unwrap_or(0);
    let mut char_idx = buffer.byte_to_char(line_start_byte);
    let next_line_byte = buffer
        .line_to_byte(line_idx + 1)
        .unwrap_or_else(|| buffer.byte_len());
    let line_end_char = buffer.byte_to_char(next_line_byte);

    while char_idx < line_end_char {
        if let Some(ch) = buffer.char_at(char_idx) {
            if !char_is_whitespace(ch) || char_is_line_ending(ch) {
                break;
            }
        }
        char_idx = next_grapheme_boundary(buffer, char_idx);
    }

    r.put_cursor(buffer, char_idx, behavior == movement::extend)
}

/// Selects the entire line(s) under the selection range (Helix `x` motion).
#[must_use]
pub fn extend_line(buffer: &rope, r: range, count: usize) -> range {
    if buffer.is_empty() {
        return range::point(0);
    }

    let cursor_pos = r.cursor(buffer);
    let current_line = buffer.byte_to_line(buffer.char_to_byte(cursor_pos));
    let max_line = buffer.line_count().saturating_sub(1);
    let target_end_line = (current_line + count).min(max_line + 1);

    let start_byte = buffer.line_to_byte(current_line).unwrap_or(0);
    let start_char = buffer.byte_to_char(start_byte);

    let end_byte = buffer
        .line_to_byte(target_end_line)
        .unwrap_or_else(|| buffer.byte_len());
    let end_char = buffer.byte_to_char(end_byte);

    range::new(start_char, end_char)
}

/// Applies a pure motion transformation to each range in a multi-selection.
#[must_use]
pub fn apply_motion(buffer: &rope, sel: &selection, f: impl Fn(range) -> range) -> selection {
    let new_ranges: Vec<range> = sel.ranges().iter().map(|&r| f(r)).collect();
    selection::new(new_ranges, sel.primary_index()).collapse(buffer)
}
