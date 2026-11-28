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
    bufferline_style, config_warning, cursor_shape, cursor_shape_config, editor_config, git_config,
    gutter_element, gutters_config, indent_guides_config, line_number_style, lsp_config,
    picker_config, search_config, soft_wrap_config, statusline_config, statusline_element,
    whitespace_config,
};
pub use keys::{
    action, default_keymaps, key_code, key_event, key_modifiers, key_trie, key_trie_node, mode,
    parse_keymaps_from_toml,
};
pub use languages::{indent_config, language_config, language_registry, lsp_server_config};
pub use merge::merge_toml;
pub use styles::{color, modifier, style};
pub use themes::{theme, theme_error};

/// Loads the standard configuration merging compiled defaults, user global, and workspace configs.
#[must_use]
pub fn load() -> ned_config {
    ned_config::load()
}

/// Loads configuration with optional explicit paths for config and runtime.
#[must_use]
pub fn load_with_paths(
    config_path: Option<&std::path::Path>,
    runtime_path: Option<&std::path::Path>,
) -> ned_config {
    ned_config::load_with_paths(config_path, runtime_path)
}

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
    /// Configuration parsing and range validation warnings.
    pub warnings: Vec<config_warning>,
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
            warnings: Vec::new(),
        }
    }

    /// Loads the full configuration, merging compiled defaults with `~/.ned/config.toml`
    /// and project `.ned/config.toml`.
    #[must_use]
    pub fn load() -> Self {
        Self::load_with_paths(None, None)
    }

    /// Loads the configuration with optional explicit paths for config and runtime.
    #[must_use]
    pub fn load_with_paths(
        config_path: Option<&std::path::Path>,
        _runtime_path: Option<&std::path::Path>,
    ) -> Self {
        let mut editor = editor_config::default();
        let mut keymaps = default_keymaps();
        let mut warnings = Vec::new();

        // Helper to load and validate a config file, capping at 1 MiB
        let mut load_file = |path: &std::path::Path, is_explicit: bool| {
            if let Ok(meta) = fs::metadata(path) {
                if meta.len() > 1024 * 1024 {
                    warnings.push(config_warning::new(format!(
                        "Configuration file '{}' exceeds 1 MiB limit; skipping",
                        path.display()
                    )));
                    return;
                }
            } else if is_explicit {
                warnings.push(config_warning::new(format!(
                    "Explicit configuration file '{}' not found",
                    path.display()
                )));
                return;
            } else {
                return;
            }

            match fs::read_to_string(path) {
                Ok(content) => match toml::from_str::<Value>(&content) {
                    Ok(val) => {
                        if let Some(editor_section) = val.get("editor") {
                            editor.apply_toml_layer(editor_section, &mut warnings);
                        }
                        if let Some(keys_section) = val.get("keys") {
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
                    Err(e) => {
                        warnings.push(config_warning::new(format!(
                            "Failed to parse '{}': {e}",
                            path.display()
                        )));
                    }
                },
                Err(e) => {
                    if is_explicit {
                        warnings.push(config_warning::new(format!(
                            "Failed to read '{}': {e}",
                            path.display()
                        )));
                    }
                }
            }
        };

        if let Some(explicit_path) = config_path {
            load_file(explicit_path, true);
        } else {
            // 1. User global ~/.ned/config.toml
            let global_path = global_config_file();
            load_file(&global_path, false);

            // 2. Workspace local .ned/config.toml
            if let Some(ws_path) = workspace_config_file() {
                load_file(&ws_path, false);
            }
        }

        // Resolve theme
        let active_theme = theme::load_or_default(&editor.theme);

        // Load language definitions
        let languages = language_registry::load();

        Self {
            editor,
            theme: active_theme,
            keys: keymaps,
            languages,
            warnings,
        }
    }
}
