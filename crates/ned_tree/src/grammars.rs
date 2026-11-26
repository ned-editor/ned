//! Language grammar definition schema and lookup methods.
//!
//! Exclusively defines the data structure loaded from JSON grammar files.

use serde::{Deserialize, Serialize};

/// Specification for lexical tokenization and syntax highlighting of a language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct grammar_def {
    /// Language name (e.g. "rust", "python", "go").
    pub name: String,
    /// Associated file extensions (e.g. `["rs"]`, `["py", "pyi"]`).
    #[serde(default)]
    pub file_types: Vec<String>,
    /// Single-line comment prefix (e.g. `"//"`, `"#"`), if any.
    #[serde(default)]
    pub line_comment: Option<String>,
    /// Multi-line block comment delimiters (e.g. `("/*", "*/")`), if any.
    #[serde(default)]
    pub block_comment: Option<(String, String)>,
    /// General keywords (e.g. `"let"`, `"mut"`, `"pub"`).
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Control-flow keywords (e.g. `"if"`, `"else"`, `"return"`, `"while"`).
    #[serde(default)]
    pub control_keywords: Vec<String>,
    /// Function declaration keywords (e.g. `"fn"`, `"def"`, `"func"`).
    #[serde(default)]
    pub function_keywords: Vec<String>,
    /// Primitive or built-in type names (e.g. `"bool"`, `"usize"`, `"int"`).
    #[serde(default)]
    pub builtin_types: Vec<String>,
    /// Whether the language supports raw string literals (`r#"..."#`).
    #[serde(default)]
    pub raw_strings: bool,
    /// Whether the language supports triple-quoted strings (`"""` or `'''`).
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

    /// Checks whether an identifier is a general keyword.
    #[must_use]
    pub fn is_keyword(&self, word: &str) -> bool {
        self.keywords.binary_search_by(|k| k.as_str().cmp(word)).is_ok()
    }

    /// Checks whether an identifier is a control-flow keyword.
    #[must_use]
    pub fn is_control_keyword(&self, word: &str) -> bool {
        self.control_keywords.binary_search_by(|k| k.as_str().cmp(word)).is_ok()
    }

    /// Checks whether an identifier is a function definition keyword.
    #[must_use]
    pub fn is_function_keyword(&self, word: &str) -> bool {
        self.function_keywords.binary_search_by(|k| k.as_str().cmp(word)).is_ok()
    }

    /// Checks whether an identifier is a primitive or built-in type.
    #[must_use]
    pub fn is_builtin_type(&self, word: &str) -> bool {
        self.builtin_types.binary_search_by(|k| k.as_str().cmp(word)).is_ok()
    }
}
