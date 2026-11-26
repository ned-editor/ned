//! Line ending representations, detection, and normalization.

use ned_rope::rope;

/// Represents one of the standard line endings.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum line_ending {
    /// CRLF ("\r\n") used on Windows.
    crlf,
    /// LF ("\n") used on Unix/Linux/macOS.
    lf,
}

impl line_ending {
    /// Native line ending for the current host operating system.
    #[cfg(target_os = "windows")]
    pub const native: line_ending = line_ending::crlf;

    /// Native line ending for the current host operating system.
    #[cfg(not(target_os = "windows"))]
    pub const native: line_ending = line_ending::lf;

    /// Returns the character length of the line ending (2 for CRLF, 1 for LF).
    #[inline]
    #[must_use]
    pub const fn len_chars(&self) -> usize {
        match self {
            Self::crlf => 2,
            Self::lf => 1,
        }
    }

    /// Returns the static string representation of this line ending.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::crlf => "\r\n",
            Self::lf => "\n",
        }
    }

    /// Determines the line ending from a character.
    #[inline]
    #[must_use]
    pub fn from_char(ch: char) -> Option<line_ending> {
        match ch {
            '\n' => Some(line_ending::lf),
            _ => None,
        }
    }
}

/// Returns `true` if the character is a line ending separator.
#[inline]
#[must_use]
pub fn char_is_line_ending(ch: char) -> bool {
    matches!(
        ch,
        '\n' | '\r' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}

/// Detects the predominant line ending in the rope buffer.
///
/// Scans the initial portion of the buffer. Defaults to native line ending
/// if the buffer is empty or contains no line breaks.
#[must_use]
pub fn detect_line_ending(buffer: &rope) -> line_ending {
    let scan_chars = buffer.char_len().min(4096);
    if scan_chars == 0 {
        return line_ending::native;
    }
    let sample = buffer.slice_to_string(0..buffer.char_to_byte(scan_chars));
    if sample.contains("\r\n") {
        line_ending::crlf
    } else if sample.contains('\n') {
        line_ending::lf
    } else {
        line_ending::native
    }
}

/// Returns the line ending present at the specified character index, if any.
#[must_use]
pub fn line_ending_at(buffer: &rope, char_idx: usize) -> Option<line_ending> {
    let ch = buffer.char_at(char_idx)?;
    if ch == '\r' {
        if let Some(next_ch) = buffer.char_at(char_idx + 1) {
            if next_ch == '\n' {
                return Some(line_ending::crlf);
            }
        }
        None
    } else if ch == '\n' {
        Some(line_ending::lf)
    } else {
        None
    }
}
