//! Editor options, statusline, gutter layout, and cursor shape configuration.

use serde::{Deserialize, Serialize};

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

/// Complete configuration for the Ned editor core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct editor_config {
    /// Active theme name (default: "default").
    #[serde(default = "default_theme_name")]
    pub theme: String,
    /// Gutter line number style (default: relative).
    #[serde(default = "default_line_number_style")]
    pub line_number: line_number_style,
    /// Highlight line containing active cursor.
    #[serde(default = "default_true")]
    pub cursor_line: bool,
    /// Enable 24-bit true color rendering.
    #[serde(default = "default_true")]
    pub true_color: bool,
    /// Default tab width.
    #[serde(default = "default_tab_width")]
    pub tab_width: usize,
    /// Number of lines of margin preserved above/below cursor when scrolling.
    #[serde(default = "default_scrolloff")]
    pub scrolloff: usize,
    /// Number of lines to scroll with scrollwheel (default: 3).
    #[serde(default = "default_scroll_lines")]
    pub scroll_lines: usize,
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
    /// Enable mouse click and scroll navigation.
    #[serde(default = "default_true")]
    pub mouse: bool,
}

impl Default for editor_config {
    fn default() -> Self {
        Self {
            theme: default_theme_name(),
            line_number: default_line_number_style(),
            cursor_line: true,
            true_color: true,
            tab_width: default_tab_width(),
            scrolloff: default_scrolloff(),
            scroll_lines: default_scroll_lines(),
            cursor_shape: cursor_shape_config::default(),
            statusline: statusline_config::default(),
            gutters: gutters_config::default(),
            search: search_config::default(),
            mouse: true,
        }
    }
}
