//! Grapheme cluster boundary navigation over rope text buffers.

use ned_rope::rope;

/// Returns `true` if the character is a combining mark, skin tone modifier,
/// zero-width joiner, or variation selector that binds to the preceding character.
#[inline]
fn is_combining_mark(ch: char) -> bool {
    let cp = ch as u32;
    (0x0300..=0x036F).contains(&cp)
        || (0x1AB0..=0x1AFF).contains(&cp)
        || (0x1DC0..=0x1DFF).contains(&cp)
        || (0x20D0..=0x20FF).contains(&cp)
        || (0xFE20..=0xFE2F).contains(&cp)
        || (0x1F3FB..=0x1F3FF).contains(&cp)
        || cp == 0x200D
        || cp == 0xFE0F
}

/// Returns `true` if the character index in the buffer represents a grapheme boundary.
#[must_use]
pub fn is_grapheme_boundary(buffer: &rope, char_idx: usize) -> bool {
    let len = buffer.char_len();
    if char_idx == 0 || char_idx >= len {
        return true;
    }

    // Check for CRLF boundary: index between \r and \n is not a boundary.
    if let Some('\n') = buffer.char_at(char_idx) {
        if let Some('\r') = buffer.char_at(char_idx - 1) {
            return false;
        }
    }

    // Check if character at char_idx is a combining mark attached to previous character.
    if let Some(ch) = buffer.char_at(char_idx) {
        if is_combining_mark(ch) {
            return false;
        }
    }

    true
}

/// Finds the next grapheme cluster boundary after `char_idx`.
#[must_use]
pub fn next_grapheme_boundary(buffer: &rope, char_idx: usize) -> usize {
    let len = buffer.char_len();
    if char_idx >= len {
        return len;
    }
    let mut pos = char_idx + 1;
    while pos < len && !is_grapheme_boundary(buffer, pos) {
        pos += 1;
    }
    pos
}

/// Finds the previous grapheme cluster boundary before `char_idx`.
#[must_use]
pub fn prev_grapheme_boundary(buffer: &rope, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }
    let len = buffer.char_len();
    let mut pos = char_idx.min(len) - 1;
    while pos > 0 && !is_grapheme_boundary(buffer, pos) {
        pos -= 1;
    }
    pos
}

/// Advances `n` grapheme cluster boundaries forward from `char_idx`.
#[must_use]
pub fn nth_next_grapheme_boundary(buffer: &rope, mut char_idx: usize, n: usize) -> usize {
    for _ in 0..n {
        let next = next_grapheme_boundary(buffer, char_idx);
        if next == char_idx {
            break;
        }
        char_idx = next;
    }
    char_idx
}

/// Moves `n` grapheme cluster boundaries backward from `char_idx`.
#[must_use]
pub fn nth_prev_grapheme_boundary(buffer: &rope, mut char_idx: usize, n: usize) -> usize {
    for _ in 0..n {
        let prev = prev_grapheme_boundary(buffer, char_idx);
        if prev == char_idx {
            break;
        }
        char_idx = prev;
    }
    char_idx
}

/// Ensures `char_idx` lies on a grapheme boundary, shifting to the next boundary if inside a cluster.
#[must_use]
pub fn ensure_grapheme_boundary_next(buffer: &rope, char_idx: usize) -> usize {
    if is_grapheme_boundary(buffer, char_idx) {
        char_idx
    } else {
        next_grapheme_boundary(buffer, char_idx)
    }
}

/// Ensures `char_idx` lies on a grapheme boundary, shifting to the previous boundary if inside a cluster.
#[must_use]
pub fn ensure_grapheme_boundary_prev(buffer: &rope, char_idx: usize) -> usize {
    if is_grapheme_boundary(buffer, char_idx) {
        char_idx
    } else {
        prev_grapheme_boundary(buffer, char_idx)
    }
}
