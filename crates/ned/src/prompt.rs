//! Helix-parity interactive command prompt with multi-column auto-completion and doc box.

pub mod completion;
pub mod editing;
pub mod history;
pub mod state;

pub use completion::{command_doc, completion_direction, COMMANDS};
pub use editing::handle_key;
pub use history::prompt_history;
pub use state::{prompt_action, prompt_state};
