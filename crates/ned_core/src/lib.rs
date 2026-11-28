//! Core editor engine, state management, document buffers, and action dispatch for ned.

#![warn(missing_docs)]

pub mod clipboard;
pub mod command_parser;
pub mod documents;
pub mod editors;
pub mod histories;
pub mod registers;
pub mod runtime_loader;
pub mod surround;
pub mod views;

pub use command_parser::{editor_command, parse_command};
pub use documents::{document, document_id};
pub use editors::editor;
pub use histories::{history, history_entry};
pub use registers::register_storage;
pub use runtime_loader::{discover_grammar_file, load_grammar};
pub use surround::{surround_add, surround_delete, surround_op, surround_replace};
pub use views::{view, view_id};
