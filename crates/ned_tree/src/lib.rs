//! Heavily optimized, pure-Rust syntax tree and token highlighting engine for ned.
//!
//! Provides zero-copy incremental lexical analysis, multiline state tracking
//! (block comments, raw strings), and scope mapping matching Helix theme conventions.

pub mod grammars;
pub mod highlighters;
pub mod lexers;
pub mod parsers;
pub mod spans;
pub mod states;
pub mod tokens;
pub mod tree;

pub use grammars::{char_class, grammar_def, grammar_rule};
pub use highlighters::syntax_highlighter;
pub use lexers::lex_line;
pub use parsers::parse_document_ast;
pub use spans::highlight_span;
pub use states::{line_state, state_cache};
pub use tokens::syntax_kind;
pub use tree::{ast_node, syntax_tree};
