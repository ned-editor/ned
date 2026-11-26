//! Core editor engine, state management, document buffers, and action dispatch for ned.

#![warn(missing_docs)]

pub mod histories;

pub use histories::{history, history_entry};
