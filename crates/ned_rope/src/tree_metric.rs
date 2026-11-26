//! metrics tracking byte length, character count, and newline count.

use std::ops::{Add, AddAssign};

/// metric metadata for a rope node or text chunk.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct metric {
    /// total length in bytes.
    pub byte_len: usize,
    /// total count of unicode scalar values (characters).
    pub char_len: usize,
    /// count of newline ('\n') characters.
    pub newlines: usize,
}

impl metric {
    /// computes metrics for a given string slice.
    #[must_use]
    pub fn of_str(text: &str) -> Self {
        let mut char_len = 0;
        let mut newlines = 0;
        for c in text.chars() {
            char_len += 1;
            if c == '\n' {
                newlines += 1;
            }
        }
        Self {
            byte_len: text.len(),
            char_len,
            newlines,
        }
    }
}

impl Add for metric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            byte_len: self.byte_len + rhs.byte_len,
            char_len: self.char_len + rhs.char_len,
            newlines: self.newlines + rhs.newlines,
        }
    }
}

impl AddAssign for metric {
    fn add_assign(&mut self, rhs: Self) {
        self.byte_len += rhs.byte_len;
        self.char_len += rhs.char_len;
        self.newlines += rhs.newlines;
    }
}
