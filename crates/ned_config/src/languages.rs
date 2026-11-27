//! Language definitions and file extension resolution for language servers.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use toml::Value;

use crate::dirs::{global_languages_file, runtime_languages_file, workspace_languages_file};
use crate::merge::merge_toml;

fn default_tab_width() -> usize {
    4
}

/// Language server command and arguments definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct lsp_server_config {
    /// Executable binary or command name.
    pub command: String,
    /// Arguments to pass to the server subprocess.
    #[serde(default)]
    pub args: Vec<String>,
}

/// Configuration options and tool associations for a single programming language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct language_config {
    /// Language name (e.g. "rust", "python", "go").
    pub name: String,
    /// Highlighting scope name (e.g. "source.rust").
    #[serde(default)]
    pub scope: Option<String>,
    /// File extensions associated with this language (e.g. ["rs"]).
    #[serde(default)]
    pub file_types: Vec<String>,
    /// Configured language server, if available.
    #[serde(default)]
    pub language_server: Option<lsp_server_config>,
    /// Default tab indentation width.
    #[serde(default = "default_tab_width")]
    pub tab_width: usize,
    /// Line comment prefix token (e.g. "//", "#").
    #[serde(default)]
    pub comment_token: Option<String>,
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

    /// Registers a language definition.
    pub fn add(&mut self, config: language_config) {
        self.languages.push(config);
    }

    /// Finds a language definition by matching file extension.
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

    /// Loads language definitions, merging built-in defaults with runtime `runtime/languages.toml`,
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
            reg.languages = wrapper.language;
        }
        reg
    }

    /// Built-in TOML value providing standard definitions for common programming languages with zero language server defaults.
    fn default_toml() -> Value {
        let toml_str = r##"
[[language]]
name = "rust"
scope = "source.rust"
file-types = ["rs"]
comment-token = "//"

[[language]]
name = "python"
scope = "source.python"
file-types = ["py", "pyi"]
comment-token = "#"

[[language]]
name = "go"
scope = "source.go"
file-types = ["go"]
comment-token = "//"

[[language]]
name = "c"
scope = "source.c"
file-types = ["c", "h"]
comment-token = "//"

[[language]]
name = "cpp"
scope = "source.cpp"
file-types = ["cpp", "cc", "cxx", "hpp", "hh", "hxx"]
comment-token = "//"

[[language]]
name = "javascript"
scope = "source.js"
file-types = ["js", "mjs", "cjs"]
comment-token = "//"

[[language]]
name = "typescript"
scope = "source.ts"
file-types = ["ts", "mts", "cts"]
comment-token = "//"

[[language]]
name = "toml"
scope = "source.toml"
file-types = ["toml"]
comment-token = "#"

[[language]]
name = "json"
scope = "source.json"
file-types = ["json"]

[[language]]
name = "markdown"
scope = "source.md"
file-types = ["md", "markdown"]
comment-token = "<!--"
"##;
        toml::from_str(toml_str).unwrap_or(Value::Table(toml::map::Map::new()))
    }
}
