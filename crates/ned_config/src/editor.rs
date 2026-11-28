//! Editor options, statusline, gutter layout, and cursor shape configuration.

use std::fmt;

use serde::{Deserialize, Serialize};
use toml::Value;

/// Display mode for line numbers in the gutter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum line_number_style {
    /// Relative line numbers with absolute line number on active line.
    relative,
    /// Absolute sequential line numbers.
    absolute,
    /// Line numbers disabled.
    none,
}

/// Visual terminal cursor shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum cursor_shape {
    /// Solid rectangular block.
    block,
    /// Vertical bar / line.
    bar,
    /// Horizontal underline.
    underline,
}

fn default_block_cursor() -> cursor_shape {
    cursor_shape::block
}

fn default_bar_cursor() -> cursor_shape {
    cursor_shape::bar
}

/// Display mode for the top bufferline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum bufferline_style {
    /// Never show bufferline.
    #[default]
    never,
    /// Always show bufferline.
    always,
    /// Show bufferline only when multiple buffers are open.
    multiple,
}

/// Configuration for cursor shape per editor mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct cursor_shape_config {
    /// Cursor shape in normal mode (default: block).
    #[serde(default = "default_block_cursor")]
    pub normal: cursor_shape,
    /// Cursor shape in insert mode (default: bar).
    #[serde(default = "default_bar_cursor")]
    pub insert: cursor_shape,
    /// Cursor shape in select mode (default: block).
    #[serde(default = "default_block_cursor")]
    pub select: cursor_shape,
}

impl Default for cursor_shape_config {
    fn default() -> Self {
        Self {
            normal: cursor_shape::block,
            insert: cursor_shape::bar,
            select: cursor_shape::block,
        }
    }
}

/// A constituent component of the statusline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum statusline_element {
    /// Current editor mode (e.g. "NOR", "INS", "SEL").
    mode,
    /// File name or path.
    #[serde(alias = "fileName")]
    file_name,
    /// Modification indicator (`[+]` if unsaved).
    #[serde(alias = "fileModificationIndicator")]
    file_modification_indicator,
    /// Diagnostic summary (errors and warnings).
    diagnostics,
    /// Cursor line and column coordinates.
    position,
    /// File encoding (e.g. "utf-8").
    #[serde(alias = "fileEncoding")]
    file_encoding,
    /// Line ending format (e.g. "LF", "CRLF").
    #[serde(alias = "fileLineEnding")]
    file_line_ending,
    /// Active selection count.
    #[serde(alias = "selectionCount")]
    selection_count,
    /// Current Git branch.
    #[serde(alias = "gitBranch")]
    git_branch,
    /// Elastic spacer filling remaining width.
    spacer,
    /// Explicit separator element.
    separator,
}

fn default_statusline_left() -> Vec<statusline_element> {
    vec![
        statusline_element::mode,
        statusline_element::file_name,
        statusline_element::file_modification_indicator,
    ]
}

fn default_statusline_right() -> Vec<statusline_element> {
    vec![
        statusline_element::diagnostics,
        statusline_element::selection_count,
        statusline_element::position,
        statusline_element::file_encoding,
        statusline_element::file_line_ending,
    ]
}

fn default_separator() -> String {
    "│".to_string()
}

/// Statusline layout configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct statusline_config {
    /// Elements aligned to the left of the statusline.
    #[serde(default = "default_statusline_left")]
    pub left: Vec<statusline_element>,
    /// Elements centered in the statusline.
    #[serde(default)]
    pub center: Vec<statusline_element>,
    /// Elements aligned to the right of the statusline.
    #[serde(default = "default_statusline_right")]
    pub right: Vec<statusline_element>,
    /// Separator string placed between elements.
    #[serde(default = "default_separator")]
    pub separator: String,
}

impl Default for statusline_config {
    fn default() -> Self {
        Self {
            left: default_statusline_left(),
            center: Vec::new(),
            right: default_statusline_right(),
            separator: default_separator(),
        }
    }
}

/// A component rendered inside the editor gutter column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum gutter_element {
    /// Git diff indicators (`+`, `~`, `-`).
    diff,
    /// Line numbers (relative or absolute).
    #[serde(alias = "lineNumbers")]
    line_numbers,
    /// LSP diagnostic markers (`E`, `W`).
    diagnostics,
    /// Padding spacer column.
    spacer,
}

fn default_gutters_layout() -> Vec<gutter_element> {
    vec![
        gutter_element::diff,
        gutter_element::diagnostics,
        gutter_element::line_numbers,
        gutter_element::spacer,
    ]
}

/// Gutter columns configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct gutters_config {
    /// Ordered list of gutter elements to render from left to right.
    #[serde(default = "default_gutters_layout")]
    pub layout: Vec<gutter_element>,
}

impl Default for gutters_config {
    fn default() -> Self {
        Self {
            layout: default_gutters_layout(),
        }
    }
}

fn default_true() -> bool {
    true
}

/// Search behavior options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct search_config {
    /// Case-insensitive search unless search pattern contains uppercase characters.
    #[serde(default = "default_true")]
    pub smart_case: bool,
    /// Wrap around document boundary when searching.
    #[serde(default = "default_true")]
    pub wrap_around: bool,
}

impl Default for search_config {
    fn default() -> Self {
        Self {
            smart_case: true,
            wrap_around: true,
        }
    }
}

/// Soft wrap configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct soft_wrap_config {
    /// Whether soft-wrapping is enabled.
    #[serde(default)]
    pub enable: bool,
}

/// Whitespace rendering configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct whitespace_config {
    /// Render policy: "none", "all", etc.
    #[serde(default = "default_whitespace_render")]
    pub render: String,
}

fn default_whitespace_render() -> String {
    "none".to_string()
}

impl Default for whitespace_config {
    fn default() -> Self {
        Self {
            render: default_whitespace_render(),
        }
    }
}

/// Indent guides rendering configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct indent_guides_config {
    /// Whether indent guides are displayed.
    #[serde(default)]
    pub render: bool,
}

/// Picker behavior configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct picker_config {
    /// Maximum candidate items to collect.
    #[serde(default = "default_picker_max_items")]
    pub max_items: usize,
    /// Maximum bytes to preview.
    #[serde(default = "default_picker_preview_bytes")]
    pub preview_max_bytes: usize,
}

fn default_picker_max_items() -> usize {
    50000
}

fn default_picker_preview_bytes() -> usize {
    1048576
}

impl Default for picker_config {
    fn default() -> Self {
        Self {
            max_items: default_picker_max_items(),
            preview_max_bytes: default_picker_preview_bytes(),
        }
    }
}

/// Language Server Protocol integration options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct lsp_config {
    /// Whether LSP client integration is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Display diagnostics published by server.
    #[serde(default = "default_true")]
    pub show_diagnostics: bool,
    /// Show automatic code completion.
    #[serde(default = "default_true")]
    pub show_completion: bool,
    /// Show signature help during parameter typing.
    #[serde(default = "default_true")]
    pub show_signature_help: bool,
    /// Display inline parameter and type hints.
    #[serde(default = "default_true")]
    pub display_inlay_hints: bool,
    /// Request and render semantic tokens.
    #[serde(default = "default_true")]
    pub semantic_tokens: bool,
}

impl Default for lsp_config {
    fn default() -> Self {
        Self {
            enabled: true,
            show_diagnostics: true,
            show_completion: true,
            show_signature_help: true,
            display_inlay_hints: true,
            semantic_tokens: true,
        }
    }
}

/// Git version control integration options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct git_config {
    /// Whether Git diff tracking and gutter marks are enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for git_config {
    fn default() -> Self {
        Self { enabled: true }
    }
}

fn default_theme_name() -> String {
    "dacctal".to_string()
}

fn default_line_number_style() -> line_number_style {
    line_number_style::relative
}

fn default_tab_width() -> usize {
    4
}

fn default_scrolloff() -> usize {
    5
}

fn default_scroll_lines() -> usize {
    3
}

/// Warning emitted during configuration parsing or validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct config_warning {
    /// Human-readable warning description.
    pub message: String,
}

impl fmt::Display for config_warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl config_warning {
    /// Constructs a new config warning.
    #[must_use]
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

/// Complete configuration for the Ned editor core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct editor_config {
    /// Active theme name (default: "dacctal").
    #[serde(default = "default_theme_name")]
    pub theme: String,
    /// Gutter line number style (default: relative).
    #[serde(default = "default_line_number_style")]
    pub line_number: line_number_style,
    /// Highlight line containing active cursor.
    #[serde(default = "default_true")]
    pub cursor_line: bool,
    /// Highlight column containing active cursor.
    #[serde(default)]
    pub cursor_column: bool,
    /// Enable 24-bit true color rendering.
    #[serde(default = "default_true")]
    pub true_color: bool,
    /// Default tab width (1..=16).
    #[serde(default = "default_tab_width")]
    pub tab_width: usize,
    /// Indentation width (1..=16).
    #[serde(default = "default_tab_width")]
    pub indent_width: usize,
    /// Insert spaces instead of hard tabs.
    #[serde(default = "default_true")]
    pub insert_spaces: bool,
    /// Number of lines of margin preserved above/below cursor when scrolling.
    #[serde(default = "default_scrolloff")]
    pub scrolloff: usize,
    /// Number of lines to scroll with scrollwheel (default: 3).
    #[serde(default = "default_scroll_lines")]
    pub scroll_lines: usize,
    /// Enable mouse click and scroll navigation.
    #[serde(default = "default_true")]
    pub mouse: bool,
    /// Top bufferline display policy.
    #[serde(default)]
    pub bufferline: bufferline_style,
    /// Visual ruler column indicators.
    #[serde(default)]
    pub rulers: Vec<usize>,
    /// Automatic closing pair insertion.
    #[serde(default = "default_true")]
    pub auto_pairs: bool,
    /// Automatic buffer saving.
    #[serde(default)]
    pub auto_save: bool,
    /// Soft-wrapping settings.
    #[serde(default)]
    pub soft_wrap: soft_wrap_config,
    /// Whitespace rendering options.
    #[serde(default)]
    pub whitespace: whitespace_config,
    /// Indent guide rendering options.
    #[serde(default)]
    pub indent_guides: indent_guides_config,
    /// Cursor shapes per mode.
    #[serde(default)]
    pub cursor_shape: cursor_shape_config,
    /// Statusline components.
    #[serde(default)]
    pub statusline: statusline_config,
    /// Gutter columns.
    #[serde(default)]
    pub gutters: gutters_config,
    /// Search behavior.
    #[serde(default)]
    pub search: search_config,
    /// Picker bounds.
    #[serde(default)]
    pub picker: picker_config,
    /// LSP settings.
    #[serde(default)]
    pub lsp: lsp_config,
    /// Git settings.
    #[serde(default)]
    pub git: git_config,
}

impl Default for editor_config {
    fn default() -> Self {
        Self {
            theme: default_theme_name(),
            line_number: default_line_number_style(),
            cursor_line: true,
            cursor_column: false,
            true_color: true,
            tab_width: default_tab_width(),
            indent_width: default_tab_width(),
            insert_spaces: true,
            scrolloff: default_scrolloff(),
            scroll_lines: default_scroll_lines(),
            mouse: true,
            bufferline: bufferline_style::default(),
            rulers: Vec::new(),
            auto_pairs: true,
            auto_save: false,
            soft_wrap: soft_wrap_config::default(),
            whitespace: whitespace_config::default(),
            indent_guides: indent_guides_config::default(),
            cursor_shape: cursor_shape_config::default(),
            statusline: statusline_config::default(),
            gutters: gutters_config::default(),
            search: search_config::default(),
            picker: picker_config::default(),
            lsp: lsp_config::default(),
            git: git_config::default(),
        }
    }
}

impl editor_config {
    /// Applies a layer of raw TOML configuration onto self, validating fields,
    /// enforcing bounds, and recording warnings for unknown or invalid values.
    pub fn apply_toml_layer(&mut self, val: &Value, warnings: &mut Vec<config_warning>) {
        let tbl = if let Value::Table(t) = val {
            t
        } else {
            warnings.push(config_warning::new("Expected table for [editor] config"));
            return;
        };

        for (k, v) in tbl {
            match k.as_str() {
                "theme" => {
                    if let Some(s) = v.as_str() {
                        if !s.is_empty()
                            && s.chars()
                                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                        {
                            self.theme = s.to_string();
                        } else {
                            warnings.push(config_warning::new(format!(
                                "Invalid editor.theme '{s}'; using default"
                            )));
                        }
                    } else {
                        warnings.push(config_warning::new("Invalid editor.theme; expected string"));
                    }
                }
                "line-number" | "line_number" => {
                    if let Some(s) = v.as_str() {
                        match s {
                            "relative" => self.line_number = line_number_style::relative,
                            "absolute" => self.line_number = line_number_style::absolute,
                            "none" => self.line_number = line_number_style::none,
                            other => warnings.push(config_warning::new(format!(
                                "Unknown editor.line-number style '{other}'"
                            ))),
                        }
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.line-number; expected string",
                        ));
                    }
                }
                "cursor-line" | "cursor_line" => {
                    if let Some(b) = v.as_bool() {
                        self.cursor_line = b;
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.cursor-line; expected bool",
                        ));
                    }
                }
                "cursor-column" | "cursor_column" => {
                    if let Some(b) = v.as_bool() {
                        self.cursor_column = b;
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.cursor-column; expected bool",
                        ));
                    }
                }
                "true-color" | "true_color" => {
                    if let Some(b) = v.as_bool() {
                        self.true_color = b;
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.true-color; expected bool",
                        ));
                    }
                }
                "tab-width" | "tab_width" => {
                    if let Some(i) = v.as_integer() {
                        if (1..=16).contains(&i) {
                            self.tab_width = i as usize;
                        } else {
                            warnings.push(config_warning::new(format!(
                                "editor.tab-width {i} out of range (1..=16)"
                            )));
                        }
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.tab-width; expected integer",
                        ));
                    }
                }
                "indent-width" | "indent_width" => {
                    if let Some(i) = v.as_integer() {
                        if (1..=16).contains(&i) {
                            self.indent_width = i as usize;
                        } else {
                            warnings.push(config_warning::new(format!(
                                "editor.indent-width {i} out of range (1..=16)"
                            )));
                        }
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.indent-width; expected integer",
                        ));
                    }
                }
                "insert-spaces" | "insert_spaces" => {
                    if let Some(b) = v.as_bool() {
                        self.insert_spaces = b;
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.insert-spaces; expected bool",
                        ));
                    }
                }
                "scrolloff" => {
                    if let Some(i) = v.as_integer() {
                        if (0..=100).contains(&i) {
                            self.scrolloff = i as usize;
                        } else {
                            warnings.push(config_warning::new(format!(
                                "editor.scrolloff {i} out of range (0..=100)"
                            )));
                        }
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.scrolloff; expected integer",
                        ));
                    }
                }
                "scroll-lines" | "scroll_lines" => {
                    if let Some(i) = v.as_integer() {
                        if (1..=100).contains(&i) {
                            self.scroll_lines = i as usize;
                        } else {
                            warnings.push(config_warning::new(format!(
                                "editor.scroll-lines {i} out of range (1..=100)"
                            )));
                        }
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.scroll-lines; expected integer",
                        ));
                    }
                }
                "mouse" => {
                    if let Some(b) = v.as_bool() {
                        self.mouse = b;
                    } else {
                        warnings.push(config_warning::new("Invalid editor.mouse; expected bool"));
                    }
                }
                "bufferline" => {
                    if let Some(s) = v.as_str() {
                        match s {
                            "never" => self.bufferline = bufferline_style::never,
                            "always" => self.bufferline = bufferline_style::always,
                            "multiple" => self.bufferline = bufferline_style::multiple,
                            other => warnings.push(config_warning::new(format!(
                                "Unknown editor.bufferline '{other}'"
                            ))),
                        }
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.bufferline; expected string",
                        ));
                    }
                }
                "rulers" => {
                    if let Some(arr) = v.as_array() {
                        let mut r_vec = Vec::new();
                        for item in arr {
                            if let Some(i) = item.as_integer() {
                                if i > 0 {
                                    r_vec.push(i as usize);
                                }
                            }
                        }
                        self.rulers = r_vec;
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.rulers; expected array of integers",
                        ));
                    }
                }
                "auto-pairs" | "auto_pairs" => {
                    if let Some(b) = v.as_bool() {
                        self.auto_pairs = b;
                    } else if v.is_table() {
                        self.auto_pairs = true;
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.auto-pairs; expected bool",
                        ));
                    }
                }
                "auto-save" | "auto_save" => {
                    if let Some(b) = v.as_bool() {
                        self.auto_save = b;
                    } else if v.is_table() {
                        self.auto_save = true;
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.auto-save; expected bool",
                        ));
                    }
                }
                "soft-wrap" | "soft_wrap" => {
                    if let Some(sub) = v.as_table() {
                        if let Some(b) = sub.get("enable").and_then(Value::as_bool) {
                            self.soft_wrap.enable = b;
                        }
                    } else if let Some(b) = v.as_bool() {
                        self.soft_wrap.enable = b;
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.soft-wrap; expected table or bool",
                        ));
                    }
                }
                "whitespace" => {
                    if let Some(sub) = v.as_table() {
                        if let Some(s) = sub.get("render").and_then(Value::as_str) {
                            self.whitespace.render = s.to_string();
                        }
                    } else if let Some(s) = v.as_str() {
                        self.whitespace.render = s.to_string();
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.whitespace; expected table or string",
                        ));
                    }
                }
                "indent-guides" | "indent_guides" => {
                    if let Some(sub) = v.as_table() {
                        if let Some(b) = sub.get("render").and_then(Value::as_bool) {
                            self.indent_guides.render = b;
                        }
                    } else if let Some(b) = v.as_bool() {
                        self.indent_guides.render = b;
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.indent-guides; expected table or bool",
                        ));
                    }
                }
                "cursor-shape" | "cursor_shape" => {
                    if let Ok(c) = v.clone().try_into::<cursor_shape_config>() {
                        self.cursor_shape = c;
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.cursor-shape configuration",
                        ));
                    }
                }
                "statusline" => {
                    if let Ok(s) = v.clone().try_into::<statusline_config>() {
                        self.statusline = s;
                    } else {
                        warnings.push(config_warning::new(
                            "Invalid editor.statusline configuration",
                        ));
                    }
                }
                "gutters" => {
                    if let Ok(g) = v.clone().try_into::<gutters_config>() {
                        self.gutters = g;
                    } else {
                        warnings.push(config_warning::new("Invalid editor.gutters configuration"));
                    }
                }
                "search" => {
                    if let Ok(s) = v.clone().try_into::<search_config>() {
                        self.search = s;
                    } else {
                        warnings.push(config_warning::new("Invalid editor.search configuration"));
                    }
                }
                "picker" => {
                    if let Ok(p) = v.clone().try_into::<picker_config>() {
                        self.picker = p;
                    } else {
                        warnings.push(config_warning::new("Invalid editor.picker configuration"));
                    }
                }
                "lsp" => {
                    if let Ok(l) = v.clone().try_into::<lsp_config>() {
                        self.lsp = l;
                    } else {
                        warnings.push(config_warning::new("Invalid editor.lsp configuration"));
                    }
                }
                "git" => {
                    if let Ok(g) = v.clone().try_into::<git_config>() {
                        self.git = g;
                    } else {
                        warnings.push(config_warning::new("Invalid editor.git configuration"));
                    }
                }
                other => {
                    warnings.push(config_warning::new(format!(
                        "Unknown configuration key 'editor.{other}'"
                    )));
                }
            }
        }
    }
}
