//! Dynamic discovery and loading of JSON syntax grammars from runtime directories.

use std::fs;
use std::path::PathBuf;

use ned_config::dirs::find_grammar_file;
use ned_tree::grammar_def;

/// Discovers the path of a grammar JSON file for `lang_name`.
///
/// Checks workspace `.ned/`, user global `~/.ned/`, and repository `runtime/grammars/` directories.
#[must_use]
pub fn discover_grammar_file(lang_name: &str) -> Option<PathBuf> {
    find_grammar_file(lang_name)
}

/// Fallback compiled grammar definitions embedded directly into the binary when `runtime/` is missing.
fn builtin_compiled_grammar(lang_name: &str) -> Option<grammar_def> {
    let json_str = match lang_name {
        "rust" => include_str!("../../../runtime/grammars/rust.json"),
        "json" => include_str!("../../../runtime/grammars/json.json"),
        "toml" => include_str!("../../../runtime/grammars/toml.json"),
        "python" => include_str!("../../../runtime/grammars/python.json"),
        "markdown" => include_str!("../../../runtime/grammars/markdown.json"),
        _ => return None,
    };
    grammar_def::from_json(json_str).ok()
}

/// Loads a language grammar definition by name from available runtime directories.
///
/// Gracefully falls back to compiled built-ins, and finally to a plain-text definition
/// if the grammar file cannot be found or read.
#[must_use]
pub fn load_grammar(lang_name: &str) -> Option<grammar_def> {
    if let Some(path) = discover_grammar_file(lang_name) {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(def) = grammar_def::from_json(&content) {
                return Some(def);
            }
        }
    }

    // Fallback 1: Compiled built-in grammar if available
    if let Some(def) = builtin_compiled_grammar(lang_name) {
        return Some(def);
    }

    // Fallback 2: Plain-text default grammar
    Some(grammar_def::new(lang_name))
}
