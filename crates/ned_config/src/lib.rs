//! Configuration, theme, keymap, and language definition loader for ned.
//!
//! Exclusively loads and manages Ned-native configurations from `~/.ned/` and `.ned/`
//! with full Helix-equivalent modal keymaps, theme scopes, and language server mappings.

pub mod dirs;
pub mod merge;
pub mod styles;

pub use dirs::{
    config_dir, find_theme_file, find_workspace_root, global_config_file, global_languages_file,
    theme_dirs, workspace_config_file, workspace_languages_file, workspace_ned_dir,
};
pub use merge::merge_toml;
pub use styles::{color, modifier, style};
