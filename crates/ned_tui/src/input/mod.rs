//! terminal input event types, ansi parser, and yokio input watcher.

/// input event models.
pub mod events;
/// ansi escape sequence parser.
pub mod parser;
/// yokio-based input watcher.
pub mod watcher;

pub use events::*;
pub use watcher::*;

/// default input handle for terminal.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultInput;
