//! Text manipulation engine for the ned editor.
//!
//! Provides fundamental data structures and algorithms for text manipulation,
//! including selection ranges, cursor positions, character categorization,
//! line endings, grapheme boundary navigation, changesets, and editor motions.

pub mod chars;
pub mod graphemes;
pub mod line_endings;

pub use chars::{categorize_char, char_category, char_is_punctuation, char_is_whitespace, char_is_word};
pub use graphemes::{
    ensure_grapheme_boundary_next, ensure_grapheme_boundary_prev, is_grapheme_boundary,
    next_grapheme_boundary, nth_next_grapheme_boundary, nth_prev_grapheme_boundary,
    prev_grapheme_boundary,
};
pub use line_endings::{char_is_line_ending, detect_line_ending, line_ending, line_ending_at};
