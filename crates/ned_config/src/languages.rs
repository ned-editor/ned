//! Language definitions, syntax grammar binding, and root resolution for language servers.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml::Value;

use crate::dirs::{global_languages_file, runtime_languages_file, workspace_languages_file};
use crate::merge::merge_toml;

fn default_tab_width() -> usize {
    4
}

fn default_indent_unit() -> String {
    "    ".to_string()
}

/// Language-specific indentation rules and token pairs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct indent_config {
    /// Number of columns per tab stop (1..=16).
    #[serde(default = "default_tab_width")]
    pub tab_width: usize,
    /// Indentation string unit (e.g. 4 spaces or tab).
    #[serde(default = "default_indent_unit")]
    pub unit: String,
    /// Tokens triggering an indentation increase.
    #[serde(default)]
    pub increase: Vec<String>,
    /// Tokens triggering an indentation decrease.
    #[serde(default)]
    pub decrease: Vec<String>,
}

impl Default for indent_config {
    fn default() -> Self {
        Self {
            tab_width: default_tab_width(),
            unit: default_indent_unit(),
            increase: Vec::new(),
            decrease: Vec::new(),
        }
    }
}

/// Language server command, arguments, and environment variables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct lsp_server_config {
    /// Executable binary or command name.
    pub command: String,
    /// Arguments to pass to the server subprocess.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables to pass to the server subprocess.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Configuration options and tool associations for a single programming language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct language_config {
    /// Language name (e.g. "rust", "python", "go").
    pub name: String,
    /// Highlighting scope name (e.g. "source.rust").
    #[serde(default)]
    pub scope: Option<String>,
    /// File extensions associated with this language (e.g. ["rs"]).
    #[serde(default, alias = "file-types")]
    pub file_types: Vec<String>,
    /// Name of grammar definition file in runtime/grammars/ (e.g. "rust").
    #[serde(default)]
    pub grammar: Option<String>,
    /// Line comment prefix token (e.g. "//", "#").
    #[serde(default, alias = "comment-token")]
    pub comment_token: Option<String>,
    /// Block comment delimiter pair (e.g. ["/*", "*/"]).
    #[serde(default, alias = "block-comment-tokens")]
    pub block_comment_tokens: Option<Vec<String>>,
    /// Indentation options and increase/decrease triggers.
    #[serde(default)]
    pub indent: Option<indent_config>,
    /// Workspace root indicator file patterns (e.g. ["Cargo.toml", ".git"]).
    #[serde(default)]
    pub roots: Vec<String>,
    /// Configured language server, if available.
    #[serde(default, alias = "language-server")]
    pub language_server: Option<lsp_server_config>,
    /// Default tab indentation width.
    #[serde(default = "default_tab_width")]
    pub tab_width: usize,
}

/// Registry managing all available language definitions and LSP associations.
#[derive(Debug, Clone, Default)]
pub struct language_registry {
    languages: Vec<language_config>,
}

impl language_registry {
    /// Creates an empty language registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            languages: Vec::new(),
        }
    }

    /// Registers a language definition, enforcing schema limits.
    pub fn add(&mut self, mut config: language_config) {
        if self.languages.len() >= 256 {
            return;
        }

        // Validate and bound field lengths
        if config.name.len() > 64 {
            config.name.truncate(64);
        }
        if let Some(ref mut sc) = config.scope {
            if sc.len() > 128 {
                sc.truncate(128);
            }
        }
        if config.file_types.len() > 64 {
            config.file_types.truncate(64);
        }
        for ft in &mut config.file_types {
            if ft.len() > 16 {
                ft.truncate(16);
            }
        }
        if config.roots.len() > 32 {
            config.roots.truncate(32);
        }

        if let Some(ref tokens) = config.block_comment_tokens {
            if tokens.len() != 2 || tokens[0].is_empty() || tokens[1].is_empty() {
                config.block_comment_tokens = None;
            }
        }

        self.languages.push(config);
    }

    /// Finds a language definition by matching file extension case-insensitively.
    #[must_use]
    pub fn detect_by_extension(&self, ext: &str) -> Option<&language_config> {
        let clean_ext = ext.strip_prefix('.').unwrap_or(ext);
        self.languages.iter().find(|l| {
            l.file_types
                .iter()
                .any(|ft| ft.eq_ignore_ascii_case(clean_ext))
        })
    }

    /// Finds a language definition from a file path.
    #[must_use]
    pub fn detect_by_path(&self, path: &Path) -> Option<&language_config> {
        let ext = path.extension()?.to_str()?;
        self.detect_by_extension(ext)
    }

    /// Finds a language definition by exact language name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&language_config> {
        self.languages
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(name))
    }

    /// Finds the workspace root for a file path based on language roots markers,
    /// falling back to Git repository root, then the file's parent directory.
    #[must_use]
    pub fn find_root_for_path(&self, path: &Path) -> Option<PathBuf> {
        let lang = self.detect_by_path(path);
        let markers = lang.map_or(&[][..], |l| l.roots.as_slice());
        let parent = path.parent()?;
        let mut cur = parent;

        while !markers.is_empty() {
            for marker in markers {
                if cur.join(marker).exists() {
                    return Some(cur.to_path_buf());
                }
            }
            match cur.parent() {
                Some(p) => cur = p,
                None => break,
            }
        }

        let mut cur_check = parent;
        loop {
            if cur_check.join(".git").exists() {
                return Some(cur_check.to_path_buf());
            }
            match cur_check.parent() {
                Some(p) => cur_check = p,
                None => break,
            }
        }

        Some(parent.to_path_buf())
    }

    /// Constructs a [`ned_lsp::server_config`] for the specified file path, if an LSP is configured.
    #[must_use]
    pub fn server_config_for(
        &self,
        path: &Path,
        root_path: Option<String>,
    ) -> Option<ned_lsp::server_config> {
        let lang = self.detect_by_path(path)?;
        let lsp = lang.language_server.as_ref()?;
        Some(ned_lsp::server_config::new(
            &lang.name,
            &lsp.command,
            lsp.args.clone(),
            root_path,
        ))
    }

    /// Loads language definitions, merging runtime `runtime/languages.toml`,
    /// global `~/.ned/languages.toml`, and project `.ned/languages.toml`.
    #[must_use]
    pub fn load() -> Self {
        let mut base_toml = Self::default_toml();

        // Merge runtime languages.toml if present (runtime/languages.toml)
        if let Some(runtime_path) = runtime_languages_file() {
            if let Ok(runtime_content) = fs::read_to_string(&runtime_path) {
                if let Ok(runtime_val) = toml::from_str::<Value>(&runtime_content) {
                    base_toml = merge_toml(base_toml, runtime_val, 3);
                }
            }
        }

        // Merge global ~/.ned/languages.toml
        let global_path = global_languages_file();
        if let Ok(global_content) = fs::read_to_string(&global_path) {
            if let Ok(global_val) = toml::from_str::<Value>(&global_content) {
                base_toml = merge_toml(base_toml, global_val, 3);
            }
        }

        // Merge workspace .ned/languages.toml
        if let Some(ws_path) = workspace_languages_file() {
            if let Ok(ws_content) = fs::read_to_string(&ws_path) {
                if let Ok(ws_val) = toml::from_str::<Value>(&ws_content) {
                    base_toml = merge_toml(base_toml, ws_val, 3);
                }
            }
        }

        Self::from_toml(base_toml)
    }

    /// Parses language registry from merged TOML value.
    #[must_use]
    pub fn from_toml(val: Value) -> Self {
        #[derive(Deserialize)]
        struct languages_wrapper {
            #[serde(default)]
            language: Vec<language_config>,
        }

        let mut reg = Self::new();
        if let Ok(wrapper) = val.try_into::<languages_wrapper>() {
            for l in wrapper.language {
                reg.add(l);
            }
        }
        reg
    }

    /// Zero built-in defaults; languages and servers are loaded strictly from runtime/ and user configs.
    fn default_toml() -> Value {
        Value::Table(toml::map::Map::new())
    }
}
