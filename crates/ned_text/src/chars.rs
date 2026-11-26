//! Utility functions to categorize unicode characters for text editing.

use crate::line_endings::char_is_line_ending;

/// Category of a character used for tokenization and motion boundaries.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum char_category {
    /// Non-line-break whitespace (spaces, tabs, unicode spaces).
    whitespace,
    /// Line ending characters (LF, CR, form feed, etc.).
    eol,
    /// Word constituent characters (alphanumeric and underscore).
    word,
    /// Punctuation and symbols.
    punctuation,
    /// Unknown or unclassified characters.
    unknown,
}

/// Categorizes a character into one of the five editing categories.
#[inline]
#[must_use]
pub fn categorize_char(ch: char) -> char_category {
    if char_is_line_ending(ch) {
        char_category::eol
    } else if char_is_whitespace(ch) {
        char_category::whitespace
    } else if char_is_word(ch) {
        char_category::word
    } else if char_is_punctuation(ch) {
        char_category::punctuation
    } else {
        char_category::unknown
    }
}

/// Returns `true` if the character is considered a word constituent.
///
/// In Helix and standard code editors, words consist of alphanumeric
/// characters and underscores.
#[inline]
#[must_use]
pub fn char_is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Returns `true` if the character is non-line-break whitespace.
#[inline]
#[must_use]
pub fn char_is_whitespace(ch: char) -> bool {
    !char_is_line_ending(ch) && ch.is_whitespace()
}

/// Returns `true` if the character is punctuation or a mathematical/currency symbol.
#[inline]
#[must_use]
pub fn char_is_punctuation(ch: char) -> bool {
    if ch == '_' {
        return false;
    }
    if ch.is_ascii_punctuation() {
        return true;
    }
    // Unicode punctuation and symbol ranges without external dependencies.
    matches!(
        ch,
        '\u{00A0}'..='\u{00BF}'
            | '\u{2000}'..='\u{206F}'
            | '\u{20A0}'..='\u{20CF}'
            | '\u{2100}'..='\u{214F}'
            | '\u{2190}'..='\u{21FF}'
            | '\u{2200}'..='\u{22FF}'
            | '\u{2300}'..='\u{23FF}'
            | '\u{25A0}'..='\u{25FF}'
            | '\u{27C0}'..='\u{27EF}'
            | '\u{2980}'..='\u{29FF}'
            | '\u{2A00}'..='\u{2AFF}'
            | '\u{2E00}'..='\u{2E7F}'
            | '\u{3000}'..='\u{303F}'
            | '\u{FF00}'..='\u{FF0F}'
            | '\u{FF1A}'..='\u{FF20}'
            | '\u{FF3B}'..='\u{FF40}'
            | '\u{FF5B}'..='\u{FF65}'
    )
}
