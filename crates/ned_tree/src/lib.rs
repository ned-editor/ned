//! Heavily optimized, pure-Rust syntax tree and token highlighting engine for ned.
//!
//! Provides zero-copy incremental lexical analysis, multiline state tracking
//! (block comments, raw strings), and scope mapping matching Helix theme conventions.
