//! Heavily optimized, pure-Rust syntax tree and token highlighting engine for ned.
//!
//! Provides zero-copy incremental lexical analysis, multiline state tracking
//! (block comments, raw strings), and scope mapping matching Helix theme conventions.

pub mod grammars;
pub mod spans;
pub mod states;
pub mod tokens;

pub use grammars::grammar_def;
pub use spans::highlight_span;
pub use states::{line_state, state_cache};
pub use tokens::syntax_kind;
