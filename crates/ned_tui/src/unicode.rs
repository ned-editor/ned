//! unicode character column width and grapheme cluster iteration.

/// returns the number of terminal columns required to display character `ch`.
#[must_use]
pub fn char_width(ch: char) -> usize {
    let cp = ch as u32;

    // control characters
    if cp <= 0x001F || (0x007F..=0x009F).contains(&cp) {
        return 0;
    }

    // zero-width combining characters and non-spacing marks
    if (0x0300..=0x036F).contains(&cp)
        || (0x1AB0..=0x1AFF).contains(&cp)
        || (0x1DC0..=0x1DFF).contains(&cp)
        || (0x20D0..=0x20FF).contains(&cp)
        || (0xFE20..=0xFE2F).contains(&cp)
        || cp == 0x200B
        || cp == 0xFEFF
    {
        return 0;
    }

    // east asian wide and fullwidth ranges
    if (0x1100..=0x115F).contains(&cp)
        || (0x2329..=0x232A).contains(&cp)
        || (0x2E80..=0x303E).contains(&cp)
        || (0x3040..=0xA4CF).contains(&cp)
        || (0xAC00..=0xD7A3).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
        || (0xFE30..=0xFE6F).contains(&cp)
        || (0xFF01..=0xFF60).contains(&cp)
        || (0xFFE0..=0xFFE6).contains(&cp)
        || (0x1F300..=0x1FAFF).contains(&cp)
        || (0x20000..=0x2FFFD).contains(&cp)
        || (0x30000..=0x3FFFD).contains(&cp)
    {
        return 2;
    }

    1
}

/// calculates display width of a string slice in terminal columns.
#[must_use]
pub fn str_width(s: &str) -> usize {
    let mut w = 0;
    for g in str_graphemes(s) {
        w += grapheme_width(g);
    }
    w
}

/// calculates terminal display width of a single grapheme cluster.
#[must_use]
pub fn grapheme_width(g: &str) -> usize {
    if g == "\t" {
        return 4;
    }
    if g == "\u{200d}" || g == "\u{fe0f}" {
        return 0;
    }
    if g.contains('\u{200d}') {
        return 2;
    }
    let mut total = 0;
    for ch in g.chars() {
        total += char_width(ch);
    }
    total.min(2)
}

/// an iterator yielding extended grapheme cluster string slices.
pub struct Graphemes<'a> {
    text: &'a str,
    byte_offset: usize,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.byte_offset >= self.text.len() {
            return None;
        }

        let remaining = &self.text[self.byte_offset..];
        let mut chars = remaining.char_indices();
        let (_, first_char) = chars.next()?;
        let mut cluster_end = first_char.len_utf8();

        // combine subsequent zero-width joiners, skin tones, or combining marks
        for (idx, next_char) in chars {
            let cp = next_char as u32;
            let is_combining = (0x0300..=0x036F).contains(&cp)
                || (0x1AB0..=0x1AFF).contains(&cp)
                || (0x1DC0..=0x1DFF).contains(&cp)
                || (0x20D0..=0x20FF).contains(&cp)
                || (0xFE20..=0xFE2F).contains(&cp)
                || (0x1F3FB..=0x1F3FF).contains(&cp) // skin tones
                || cp == 0x200D // zwj
                || cp == 0xFE0F; // variation selector

            if is_combining {
                cluster_end = idx + next_char.len_utf8();
            } else {
                break;
            }
        }

        let slice = &remaining[..cluster_end];
        self.byte_offset += cluster_end;
        Some(slice)
    }
}

/// splits string into grapheme cluster slices.
#[must_use]
pub fn str_graphemes(s: &str) -> Graphemes<'_> {
    Graphemes {
        text: s,
        byte_offset: 0,
    }
}
