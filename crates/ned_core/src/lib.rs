//! Core editor engine, state management, document buffers, and action dispatch for ned.

#![warn(missing_docs)]

pub mod histories;
pub mod registers;
pub mod runtime_loader;

pub use histories::{history, history_entry};
pub use registers::register_storage;
pub use runtime_loader::{discover_grammar_file, load_grammar};
