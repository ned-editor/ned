//! # `ned_regex`
//!
//! A dedicated, bounded Thompson-NFA regular expression engine engineered for Ned.
//! Provides deterministic linear-time matching ($O(m \times n)$) without catastrophic
//! backtracking or external dependencies.

pub mod compiler;
pub mod matcher;
pub mod parser;

pub use compiler::nfa_program;
pub use matcher::regex_match;
pub use parser::regex_error;

use compiler::pattern_compiler;
use matcher::pattern_matcher;
use parser::pattern_parser;

/// Compiled regular expression pattern ready for matching.
#[derive(Clone, Debug)]
pub struct regex {
    /// Original pattern string.
    pattern: String,
    /// Compiled NFA program.
    program: nfa_program,
}

impl regex {
    /// Compiles `pattern` with default case-sensitive matching.
    ///
    /// # Errors
    ///
    /// Returns a `regex_error` if `pattern` is malformed.
    pub fn new(pattern: &str) -> Result<Self, regex_error> {
        Self::with_options(pattern, false)
    }

    /// Compiles `pattern` with explicit case-insensitivity option.
    ///
    /// # Errors
    ///
    /// Returns a `regex_error` if `pattern` is malformed.
    pub fn with_options(pattern: &str, case_insensitive: bool) -> Result<Self, regex_error> {
        let mut p = pattern_parser::new(pattern);
        let ast = p.parse()?;
        let program = pattern_compiler::compile(&ast, case_insensitive);
        Ok(Self {
            pattern: pattern.to_string(),
            program,
        })
    }

    /// Compiles `pattern` using smart-case heuristic:
    /// case-insensitive unless `pattern` contains any uppercase character.
    ///
    /// # Errors
    ///
    /// Returns a `regex_error` if `pattern` is malformed.
    pub fn smart_case(pattern: &str) -> Result<Self, regex_error> {
        let has_uppercase = pattern.chars().any(char::is_uppercase);
        Self::with_options(pattern, !has_uppercase)
    }

    /// Returns the original string representation of the pattern.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.pattern
    }

    /// Searches for the first match within `haystack`.
    #[inline]
    #[must_use]
    pub fn find(&self, haystack: &str) -> Option<regex_match> {
        self.find_at(haystack, 0)
    }

    /// Searches for the first match in `haystack` starting at or after `start_byte`.
    #[inline]
    #[must_use]
    pub fn find_at(&self, haystack: &str, start_byte: usize) -> Option<regex_match> {
        let m = pattern_matcher::new(&self.program);
        m.find_at(haystack, start_byte)
    }

    /// Returns an iterator over all non-overlapping matches in `haystack`.
    #[inline]
    #[must_use]
    pub fn find_iter<'a, 'p>(&'p self, haystack: &'a str) -> regex_matches<'a, 'p> {
        regex_matches {
            re: self,
            haystack,
            cursor: 0,
        }
    }

    /// Returns the last match ending at or before `before_byte`.
    /// Useful for backward search (`?` or `search_prev`).
    #[must_use]
    pub fn rfind(&self, haystack: &str, before_byte: usize) -> Option<regex_match> {
        let mut last = None;
        for m in self.find_iter(haystack) {
            if m.start <= before_byte {
                last = Some(m);
            } else {
                break;
            }
        }
        last
    }

    /// Tests whether the pattern matches anywhere within `haystack`.
    #[inline]
    #[must_use]
    pub fn is_match(&self, haystack: &str) -> bool {
        self.find(haystack).is_some()
    }

    /// Returns an iterator yielding substrings separated by matches of this pattern.
    #[inline]
    #[must_use]
    pub fn split<'a, 'p>(&'p self, haystack: &'a str) -> regex_split<'a, 'p> {
        regex_split {
            matches: self.find_iter(haystack),
            haystack,
            last_end: 0,
            finished: false,
        }
    }

    /// Replaces all occurrences of this pattern in `haystack` with `replacement`.
    #[must_use]
    pub fn replace_all(&self, haystack: &str, replacement: &str) -> String {
        let mut result = String::with_capacity(haystack.len());
        let mut last_end = 0;

        for m in self.find_iter(haystack) {
            result.push_str(&haystack[last_end..m.start]);
            result.push_str(replacement);
            last_end = m.end;
        }

        result.push_str(&haystack[last_end..]);
        result
    }
}

/// Iterator yielding non-overlapping matches of a `regex`.
pub struct regex_matches<'a, 'p> {
    /// The regex being matched.
    re: &'p regex,
    /// The input being searched.
    haystack: &'a str,
    /// Current search cursor position.
    cursor: usize,
}

impl Iterator for regex_matches<'_, '_> {
    type Item = regex_match;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor > self.haystack.len() {
            return None;
        }

        let m = self.re.find_at(self.haystack, self.cursor)?;

        if m.start == m.end {
            // Zero-width match; advance cursor by at least 1 character to avoid infinite loop
            if let Some((_, next_char)) = self.haystack[m.end..].char_indices().next() {
                self.cursor = m.end + next_char.len_utf8();
            } else {
                self.cursor = self.haystack.len() + 1;
            }
        } else {
            self.cursor = m.end;
        }

        Some(m)
    }
}

/// Iterator yielding substrings separated by `regex` matches.
pub struct regex_split<'a, 'p> {
    /// Underlying match iterator.
    matches: regex_matches<'a, 'p>,
    /// The input being split.
    haystack: &'a str,
    /// End of the last emitted match.
    last_end: usize,
    /// Whether iteration is complete.
    finished: bool,
}

impl<'a> Iterator for regex_split<'a, '_> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        if let Some(m) = self.matches.next() {
            let part = &self.haystack[self.last_end..m.start];
            self.last_end = m.end;
            Some(part)
        } else {
            self.finished = true;
            Some(&self.haystack[self.last_end..])
        }
    }
}

/// Builder for configuring and compiling a `regex`.
#[derive(Clone, Debug, Default)]
pub struct regex_builder {
    /// Whether matching is case-insensitive.
    case_insensitive: bool,
}

impl regex_builder {
    /// Creates a new default builder.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether matching is case-insensitive.
    #[inline]
    #[must_use]
    pub fn case_insensitive(mut self, yes: bool) -> Self {
        self.case_insensitive = yes;
        self
    }

    /// Compiles `pattern` with configured options.
    ///
    /// # Errors
    ///
    /// Returns a `regex_error` if `pattern` is malformed.
    #[inline]
    pub fn build(self, pattern: &str) -> Result<regex, regex_error> {
        regex::with_options(pattern, self.case_insensitive)
    }
}
