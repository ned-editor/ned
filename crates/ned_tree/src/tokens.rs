//! Syntax token kinds and theme scope mappings.

/// Classification of a syntax token matching standard theme highlighting scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum syntax_kind {
    /// Generic language keyword (e.g. `let`, `pub`, `def`, `class`).
    keyword,
    /// Control flow keyword (e.g. `if`, `else`, `return`, `match`, `for`, `while`).
    keyword_control,
    /// Function declaration keyword (e.g. `fn`, `def`, `func`, `function`).
    keyword_function,
    /// User-defined type or struct name (e.g. `String`, `Option`, `MyStruct`).
    type_name,
    /// Primitive or built-in type name (e.g. `u32`, `i64`, `bool`, `int`, `str`).
    type_builtin,
    /// Function declaration or definition name.
    function_name,
    /// Function or method invocation name.
    function_call,
    /// Variable or identifier name.
    variable_name,
    /// String literal (e.g. `"hello world"`, `'hello'`).
    string_literal,
    /// Numeric constant (e.g. `123`, `0x4f`, `3.1415`).
    number_literal,
    /// Character literal (e.g. `'c'`, `'\n'`).
    char_literal,
    /// Single-line comment (e.g. `// comment`, `# comment`).
    comment_line,
    /// Multi-line or block comment (e.g. `/* comment */`, `""" doc """`).
    comment_block,
    /// Operator symbol (e.g. `+`, `-`, `*`, `==`, `!=`, `&&`, `||`).
    operator,
    /// Structural punctuation (e.g. `(`, `)`, `{`, `}`, `[`, `]`, `;`, `,`).
    punctuation,
    /// Attribute, decorator, or macro (e.g. `#[derive]`, `@decorator`, `println!`).
    attribute,
    /// Standard unhighlighted body text.
    text,
}

impl syntax_kind {
    /// Returns the standard theme scope path corresponding to this syntax token.
    ///
    /// Directly maps to Helix theme scope conventions (`"keyword"`, `"string"`, `"comment"`, etc.).
    #[must_use]
    pub const fn as_scope(&self) -> &'static str {
        match self {
            Self::keyword => "keyword",
            Self::keyword_control => "keyword.control",
            Self::keyword_function => "keyword.function",
            Self::type_name => "type",
            Self::type_builtin => "type.builtin",
            Self::function_name | Self::function_call => "function",
            Self::variable_name => "variable",
            Self::string_literal => "string",
            Self::number_literal => "constant.numeric",
            Self::char_literal => "string.special",
            Self::comment_line => "comment.line",
            Self::comment_block => "comment.block",
            Self::operator => "operator",
            Self::punctuation => "punctuation",
            Self::attribute => "attribute",
            Self::text => "ui.text",
        }
    }
}
