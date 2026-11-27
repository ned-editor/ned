//! Language grammar definition schema and declarative rule primitives.
//!
//! Exclusively defines language-agnostic data structures loaded from JSON grammar files.

use serde::{Deserialize, Serialize};

use crate::tokens::syntax_kind;

/// Default serde helper returning `true`.
fn default_true() -> bool {
    true
}

/// Character class for matching token bodies in declarative pattern rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum char_class {
    /// Identifier characters: `[a-zA-Z_][a-zA-Z0-9_]*`.
    ident,
    /// Numeric characters: `[0-9][0-9a-zA-Z._]*`.
    digits,
    /// Any non-whitespace characters.
    non_whitespace,
}

/// Declarative lexical/AST rule evaluated in priority order by the parser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum grammar_rule {
    /// Delimited span from `begin` to `end` (e.g., comments, strings, attributes).
    span {
        /// Opening delimiter string (e.g., `"/*"`, `"\""`, `"//"`).
        begin: String,
        /// Closing delimiter string (e.g., `"*/"`, `"\""`, `"\n"`).
        end: String,
        /// Token kind produced for this span.
        kind: syntax_kind,
        /// Optional escape character ignoring the next character (e.g., `'\\'`).
        #[serde(default)]
        escape: Option<char>,
        /// Whether this span can cross line breaks (defaults to `true`).
        #[serde(default = "default_true")]
        multiline: bool,
        /// Whether nested `begin` delimiters increase depth.
        #[serde(default)]
        nested: bool,
    },
    /// Paired delimiter with variable repeat count (e.g., `r#"..."#`, `[=[...]=]`, ```` ``` ````).
    paired_span {
        /// Optional fixed prefix before repeat characters (e.g., `"r"`).
        #[serde(default)]
        prefix: String,
        /// Character that can repeat N times (e.g., `'#'` or `'='`).
        repeat_char: char,
        /// Minimum number of repeat characters required.
        #[serde(default)]
        min_repeats: usize,
        /// Opening delimiter after repeats (e.g., `"\""`).
        #[serde(default)]
        open: String,
        /// Closing delimiter before matching N repeats (e.g., `"\""`).
        #[serde(default)]
        close: String,
        /// Token kind produced for this span.
        kind: syntax_kind,
    },
    /// Pattern rule matching an optional prefix, character class body, and optional suffix.
    pattern {
        /// Optional literal prefix (e.g., `"'"`, `"@"`, `"$"`).
        #[serde(default)]
        prefix: Option<String>,
        /// Character class for the token body.
        body: char_class,
        /// Optional literal suffix (e.g., `"!"`).
        #[serde(default)]
        suffix: Option<String>,
        /// Optional character that must NOT immediately follow the match.
        #[serde(default)]
        not_followed_by: Option<char>,
        /// Token kind produced when matched.
        kind: syntax_kind,
    },
}

/// Specification for lexical tokenization and AST parsing of a language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct grammar_def {
    /// Language name (e.g. `"rust"`, `"python"`, `"go"`).
    pub name: String,
    /// Associated file extensions (e.g. `["rs"]`, `["go"]`).
    #[serde(default)]
    pub file_types: Vec<String>,
    /// Declarative grammar rules evaluated in order.
    #[serde(default)]
    pub rules: Vec<grammar_rule>,
    /// Optional shorthand single-line comment prefix (e.g. `"//"`, `"#"`).
    #[serde(default)]
    pub line_comment: Option<String>,
    /// Optional shorthand multi-line block comment delimiters (e.g. `("/*", "*/")`).
    #[serde(default)]
    pub block_comment: Option<(String, String)>,
    /// General keywords (e.g. `"let"`, `"var"`, `"package"`).
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Control-flow keywords (e.g. `"if"`, `"else"`, `"return"`, `"switch"`).
    #[serde(default)]
    pub control_keywords: Vec<String>,
    /// Function declaration keywords (e.g. `"fn"`, `"def"`, `"func"`).
    #[serde(default)]
    pub function_keywords: Vec<String>,
    /// Primitive or built-in type names (e.g. `"bool"`, `"int"`, `"string"`).
    #[serde(default)]
    pub builtin_types: Vec<String>,
    /// Legacy shorthand flag for raw strings.
    #[serde(default)]
    pub raw_strings: bool,
    /// Legacy shorthand flag for triple-quoted strings.
    #[serde(default)]
    pub triple_strings: bool,
}

impl grammar_def {
    /// Creates a new empty grammar definition.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Deserializes a grammar definition from a JSON string and sorts its keyword vectors.
    ///
    /// # Errors
    ///
    /// Returns [`serde_json::Error`] if the JSON is malformed.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let mut def: Self = serde_json::from_str(json_str)?;
        def.sort_keywords();
        Ok(def)
    }

    /// Sorts all keyword vectors alphabetically to enable $O(\log N)$ binary search.
    pub fn sort_keywords(&mut self) {
        self.keywords.sort_unstable();
        self.keywords.dedup();
        self.control_keywords.sort_unstable();
        self.control_keywords.dedup();
        self.function_keywords.sort_unstable();
        self.function_keywords.dedup();
        self.builtin_types.sort_unstable();
        self.builtin_types.dedup();
    }

    /// Returns the active list of declarative grammar rules, synthesizing defaults from
    /// shorthand fields if `rules` is empty.
    #[must_use]
    pub fn effective_rules(&self) -> Vec<grammar_rule> {
        if !self.rules.is_empty() {
            return self.rules.clone();
        }

        let mut out = Vec::new();
        if let Some((ref start, ref end)) = self.block_comment {
            if !start.is_empty() && !end.is_empty() {
                out.push(grammar_rule::span {
                    begin: start.clone(),
                    end: end.clone(),
                    kind: syntax_kind::comment_block,
                    escape: None,
                    multiline: true,
                    nested: false,
                });
            }
        }
        if let Some(ref prefix) = self.line_comment {
            if !prefix.is_empty() {
                out.push(grammar_rule::span {
                    begin: prefix.clone(),
                    end: "\n".to_string(),
                    kind: syntax_kind::comment_line,
                    escape: None,
                    multiline: false,
                    nested: false,
                });
            }
        }
        if self.triple_strings {
            out.push(grammar_rule::span {
                begin: "\"\"\"".to_string(),
                end: "\"\"\"".to_string(),
                kind: syntax_kind::string_literal,
                escape: Some('\\'),
                multiline: true,
                nested: false,
            });
            out.push(grammar_rule::span {
                begin: "'''".to_string(),
                end: "'''".to_string(),
                kind: syntax_kind::string_literal,
                escape: Some('\\'),
                multiline: true,
                nested: false,
            });
        }
        out.push(grammar_rule::span {
            begin: "\"".to_string(),
            end: "\"".to_string(),
            kind: syntax_kind::string_literal,
            escape: Some('\\'),
            multiline: true,
            nested: false,
        });
        out
    }

    /// Checks whether an identifier is a general keyword.
    #[must_use]
    pub fn is_keyword(&self, word: &str) -> bool {
        self.keywords
            .binary_search_by(|k| k.as_str().cmp(word))
            .is_ok()
    }

    /// Checks whether an identifier is a control-flow keyword.
    #[must_use]
    pub fn is_control_keyword(&self, word: &str) -> bool {
        self.control_keywords
            .binary_search_by(|k| k.as_str().cmp(word))
            .is_ok()
    }

    /// Checks whether an identifier is a function definition keyword.
    #[must_use]
    pub fn is_function_keyword(&self, word: &str) -> bool {
        self.function_keywords
            .binary_search_by(|k| k.as_str().cmp(word))
            .is_ok()
    }

    /// Checks whether an identifier is a primitive or built-in type.
    #[must_use]
    pub fn is_builtin_type(&self, word: &str) -> bool {
        self.builtin_types
            .binary_search_by(|k| k.as_str().cmp(word))
            .is_ok()
    }
}
