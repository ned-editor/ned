//! Modal keymaps, key event parsing, and editor action definitions.
//!
//! Replicates Helix modal editing key semantics (Normal, Insert, Select modes,
//! `g` goto sub-layer, `[` and `]` brackets) using Ned's native types.

pub mod actions;
pub mod defaults;
pub mod key_types;
pub mod parser;
pub mod trie;

pub use actions::action;
pub use defaults::default_keymaps;
pub use key_types::{key_code, key_event, key_modifiers, mode};
pub use parser::parse_keymaps_from_toml;
pub use trie::{key_trie, key_trie_node};
