//! Helix-parity interactive command prompt with multi-column auto-completion and doc box.

pub mod completion;
pub mod editing;
pub mod state;

pub use completion::{command_doc, completion_direction, COMMANDS};
pub use editing::handle_key;
pub use state::{prompt_action, prompt_state};
