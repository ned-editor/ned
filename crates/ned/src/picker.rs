//! Interactive file and buffer picker popup with fuzzy filtering and live preview pane.

pub mod filesystem;
pub mod filter;
pub mod preview;
pub mod state;

pub use filesystem::collect_files;
pub use filter::{filter_candidates, find_subsequence_matches};
pub use preview::{max_file_size_for_preview, min_area_width_for_preview, preview_content};
pub use state::{picker_action, picker_state};
