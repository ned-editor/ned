//! Dynamic discovery and loading of JSON syntax grammars from runtime directories.

use std::fs;
use std::path::PathBuf;

use ned_config::dirs::{config_dir, workspace_ned_dir};
use ned_tree::grammar_def;

/// Discovers the path of a grammar JSON file for `lang_name`.
///
/// Checks workspace `.ned/runtime/grammars/<lang_name>.json` first,
/// falling back to user global `~/.ned/runtime/grammars/<lang_name>.json`.
#[must_use]
pub fn discover_grammar_file(lang_name: &str) -> Option<PathBuf> {
    let filename = format!("{lang_name}.json");

    // 1. Workspace override
    if let Some(ws_ned) = workspace_ned_dir() {
        let ws_path = ws_ned.join("runtime").join("grammars").join(&filename);
        if ws_path.is_file() {
            return Some(ws_path);
        }
    }

    // 2. Global runtime
    let global_path = config_dir().join("runtime").join("grammars").join(&filename);
    if global_path.is_file() {
        return Some(global_path);
    }

    None
}

/// Loads a language grammar definition by name from available runtime directories.
///
/// Returns `None` if the grammar file cannot be found, read, or parsed.
#[must_use]
pub fn load_grammar(lang_name: &str) -> Option<grammar_def> {
    let path = discover_grammar_file(lang_name)?;
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}
