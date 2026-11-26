//! Ned main editor binary library and runtime orchestrator.

#![warn(missing_docs)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]

pub mod app;
pub mod input_mapping;
pub mod prompt;
pub mod renderer;
pub mod theme_bridge;

pub use app::run;
