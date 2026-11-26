//! Core editor engine, state management, document buffers, and action dispatch for ned.

#![warn(missing_docs)]

pub mod documents;
pub mod histories;
pub mod registers;
pub mod runtime_loader;
pub mod views;

pub use documents::{document, document_id};
pub use histories::{history, history_entry};
pub use registers::register_storage;
pub use runtime_loader::{discover_grammar_file, load_grammar};
pub use views::{view, view_id};
