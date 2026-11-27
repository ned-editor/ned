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

use std::collections::HashMap;
use std::fs;

use toml::Value;

pub use dirs::{
    config_dir, find_theme_file, find_workspace_root, global_config_file, global_languages_file,
    runtime_languages_file, theme_dirs, workspace_config_file, workspace_languages_file,
    workspace_ned_dir,
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

/// Top-level aggregated configuration for the Ned editor session.
#[derive(Debug, Clone)]
pub struct ned_config {
    /// Editor behavior, cursor, statusline, and gutter options.
    pub editor: editor_config,
    /// Resolved color theme.
    pub theme: theme,
    /// Keybinding trees per editor mode.
    pub keys: HashMap<mode, key_trie_node>,
    /// Language definitions and LSP server associations.
    pub languages: language_registry,
}

impl ned_config {
    /// Constructs defaults with zero filesystem dependencies.
    #[must_use]
    pub fn defaults() -> Self {
        Self {
            editor: editor_config::default(),
            theme: theme::default_dark(),
            keys: default_keymaps(),
            languages: language_registry::load(),
        }
    }

    /// Loads the full configuration, merging compiled defaults with `~/.ned/config.toml`
    /// and project `.ned/config.toml`.
    #[must_use]
    pub fn load() -> Self {
        let mut editor_val = Value::Table(toml::map::Map::new());
        let mut keymaps = default_keymaps();

        // 1. Load global ~/.ned/config.toml if present
        let global_path = global_config_file();
        if let Ok(global_str) = fs::read_to_string(&global_path) {
            if let Ok(global_toml) = toml::from_str::<Value>(&global_str) {
                if let Some(editor_section) = global_toml.get("editor") {
                    editor_val = merge_toml(editor_val, editor_section.clone(), 3);
                }
                if let Some(keys_section) = global_toml.get("keys") {
                    let user_keys = parse_keymaps_from_toml(keys_section);
                    for (m, user_trie) in user_keys {
                        if let Some(existing_trie) = keymaps.get_mut(&m) {
                            existing_trie.merge(user_trie);
                        } else {
                            keymaps.insert(m, user_trie);
                        }
                    }
                }
            }
        }

        // 2. Load workspace .ned/config.toml if present
        if let Some(ws_path) = workspace_config_file() {
            if let Ok(ws_str) = fs::read_to_string(&ws_path) {
                if let Ok(ws_toml) = toml::from_str::<Value>(&ws_str) {
                    if let Some(editor_section) = ws_toml.get("editor") {
                        editor_val = merge_toml(editor_val, editor_section.clone(), 3);
                    }
                    if let Some(keys_section) = ws_toml.get("keys") {
                        let user_keys = parse_keymaps_from_toml(keys_section);
                        for (m, user_trie) in user_keys {
                            if let Some(existing_trie) = keymaps.get_mut(&m) {
                                existing_trie.merge(user_trie);
                            } else {
                                keymaps.insert(m, user_trie);
                            }
                        }
                    }
                }
            }
        }

        // Deserialize editor_config from merged TOML or fall back to defaults
        let editor: editor_config = editor_val
            .try_into()
            .unwrap_or_else(|_| editor_config::default());

        // Resolve theme
        let active_theme = theme::load_or_default(&editor.theme);

        // Load language definitions
        let languages = language_registry::load();

        Self {
            editor,
            theme: active_theme,
            keys: keymaps,
            languages,
        }
    }
}
