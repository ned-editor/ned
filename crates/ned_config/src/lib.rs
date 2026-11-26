//! Configuration, theme, keymap, and language definition loader for ned.
//!
//! Exclusively loads and manages Ned-native configurations from `~/.ned/` and `.ned/`
//! with full Helix-equivalent modal keymaps, theme scopes, and language server mappings.

pub mod dirs;
pub mod editor;
pub mod keys;
pub mod languages;
pub mod merge;
pub mod styles;
pub mod themes;

pub use dirs::{
    config_dir, find_theme_file, find_workspace_root, global_config_file, global_languages_file,
    theme_dirs, workspace_config_file, workspace_languages_file, workspace_ned_dir,
};
pub use editor::{
    cursor_shape, cursor_shape_config, editor_config, gutter_element, gutters_config,
    line_number_style, search_config, statusline_config, statusline_element,
};
pub use keys::{
    action, default_keymaps, key_code, key_event, key_modifiers, key_trie, key_trie_node, mode,
    parse_keymaps_from_toml,
};
pub use languages::{language_config, language_registry, lsp_server_config};
pub use merge::merge_toml;
pub use styles::{color, modifier, style};
pub use themes::{theme, theme_error};
