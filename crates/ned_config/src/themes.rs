//! Theme loading, palette resolution, and hierarchical scope styling.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;

use toml::Value;

use crate::dirs::find_theme_file;
use crate::merge::merge_toml;
use crate::styles::{color, modifier, style};

/// Error encountered while loading or parsing a theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum theme_error {
    /// Failed to read theme file from disk.
    io_error(String),
    /// Failed to parse theme TOML.
    parse_error(String),
    /// Theme inheritance loop detected.
    cycle_detected(String),
}

impl fmt::Display for theme_error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::io_error(e) => write!(f, "theme IO error: {e}"),
            Self::parse_error(e) => write!(f, "theme parse error: {e}"),
            Self::cycle_detected(name) => write!(f, "theme inheritance cycle detected at '{name}'"),
        }
    }
}

impl std::error::Error for theme_error {}

/// A fully resolved theme containing scope-to-style mappings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct theme {
    /// Human-readable theme name.
    pub name: String,
    styles: HashMap<String, style>,
}

impl theme {
    /// Creates a new empty theme.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            styles: HashMap::new(),
        }
    }

    /// Sets the style for a specific scope.
    pub fn insert(&mut self, scope: impl Into<String>, s: style) {
        self.styles.insert(scope.into(), s);
    }

    /// Looks up a style by scope, applying parent scope fallback if exact scope is missing.
    ///
    /// For example, querying `"ui.cursor.insert"` will check:
    /// 1. `"ui.cursor.insert"`
    /// 2. `"ui.cursor"`
    /// 3. `"ui"`
    /// 4. Falls back to an empty style if none match.
    #[must_use]
    pub fn get(&self, scope: &str) -> style {
        let mut current_scope = scope;
        loop {
            if let Some(s) = self.styles.get(current_scope) {
                return *s;
            }
            match current_scope.rfind('.') {
                Some(idx) => current_scope = &current_scope[..idx],
                None => break,
            }
        }
        style::new()
    }

    /// Loads a theme by name from the configured theme directories (`.ned/themes/` then `~/.ned/themes/`).
    ///
    /// Falls back to the built-in default theme `dacctal` if `name` is `"default"`, `"dacctal"`, or if the file cannot be located.
    #[must_use]
    pub fn load_or_default(name: &str) -> Self {
        if name == "default" || name == "dacctal" || name == "ned_dark" {
            return Self::default_dacctal();
        }

        Self::load(name).unwrap_or_else(|_| Self::default_dacctal())
    }

    /// Loads a theme by name with inheritance support.
    ///
    /// # Errors
    ///
    /// Returns [`theme_error`] if file is missing, invalid TOML, or contains an inheritance cycle.
    pub fn load(name: &str) -> Result<Self, theme_error> {
        let mut visited = HashSet::new();
        let value = load_theme_value(name, &mut visited)?;
        parse_theme_from_value(name, value)
    }

    /// Built-in `dacctal` theme: organic, minimalist, and premium dark aesthetic.
    #[must_use]
    pub fn default_dacctal() -> Self {
        let mut t = Self::new("dacctal");

        // Named palette derived from dacctal custom stylesheet
        let obsidian = color::rgb(15, 15, 15); // #0f0f0f - --cl-black
        let basalt = color::rgb(20, 21, 20); // #141514 - --cl-gray-0
        let coal = color::rgb(30, 31, 30); // #1e1f1e - --cl-gray-1
        let slate = color::rgb(39, 42, 40); // #272a28 - --cl-gray-2
        let pebble = color::rgb(59, 64, 60); // #3b403c - --cl-gray-3
        let flint = color::rgb(88, 95, 91); // #585f5b - --cl-gray-4
        let ash = color::rgb(108, 117, 111); // #6c756f - --cl-gray-5
        let lichen = color::rgb(136, 142, 123); // #888e7b - --cl-gray-6
        let moss = color::rgb(154, 156, 139); // #9a9c8b - --cl-gray-7
        let sandstone = color::rgb(182, 182, 154); // #b6b69a - --cl-gray-8
        let clay = color::rgb(227, 214, 201); // #e3d6c9 - --cl-gray-10
        let parchment = color::rgb(244, 222, 205); // #f4decd - --cl-white

        let cinnabar = color::rgb(241, 110, 101); // #f16e65 - --cl-red-6
        let terracotta = color::rgb(239, 147, 77); // #ef934d - --cl-orange-6
        let apricot = color::rgb(244, 181, 133); // #f4b585 - --cl-orange-7
        let amber = color::rgb(239, 191, 113); // #efbf71 - --cl-yellow
        let sage = color::rgb(126, 201, 126); // #7ec97e - --cl-green-6
        let mint = color::rgb(164, 218, 164); // #a4daa4 - --cl-green-7
        let verdigris = color::rgb(126, 201, 176); // #7ec9b0 - --cl-cyan-6
        let cerulean = color::rgb(113, 180, 214); // #71b4d6 - --cl-blue-6
        let glacier = color::rgb(176, 212, 232); // #b0d4e8 - --cl-blue-8
        let heather = color::rgb(226, 141, 198); // #e28dc6 - --cl-magenta-7

        // UI scopes
        t.insert("ui.background", style::new().bg(obsidian));
        t.insert("ui.text", style::new().fg(parchment));
        t.insert(
            "ui.text.focus",
            style::new().fg(parchment).modifier(modifier {
                bold: true,
                ..Default::default()
            }),
        );
        t.insert("ui.cursor", style::new().fg(obsidian).bg(parchment));
        t.insert("ui.cursor.normal", style::new().fg(obsidian).bg(parchment));
        t.insert("ui.cursor.insert", style::new().fg(obsidian).bg(mint));
        t.insert("ui.cursor.select", style::new().fg(obsidian).bg(glacier));
        t.insert(
            "ui.cursor.primary",
            style::new().fg(obsidian).bg(parchment).modifier(modifier {
                bold: true,
                ..Default::default()
            }),
        );
        t.insert("ui.selection", style::new().bg(pebble));
        t.insert("ui.selection.primary", style::new().bg(flint));
        t.insert("ui.linenr", style::new().fg(ash));
        t.insert(
            "ui.linenr.selected",
            style::new().fg(clay).modifier(modifier {
                bold: true,
                ..Default::default()
            }),
        );
        t.insert("ui.statusline", style::new().fg(parchment).bg(basalt));
        t.insert("ui.statusline.inactive", style::new().fg(ash).bg(basalt));
        t.insert("ui.gutter", style::new().bg(obsidian));
        t.insert("ui.popup", style::new().fg(parchment).bg(coal));
        t.insert("ui.menu", style::new().fg(parchment).bg(coal));
        t.insert("ui.menu.selected", style::new().fg(obsidian).bg(terracotta));
        t.insert("ui.window", style::new().fg(slate));
        t.insert("ui.background.separator", style::new().fg(slate));
        t.insert("ui.help", style::new().fg(parchment).bg(coal));
        t.insert("ui.text.inactive", style::new().fg(ash));
        t.insert("ui.highlight", style::new().bg(flint));
        t.insert(
            "special",
            style::new().fg(amber).modifier(modifier {
                bold: true,
                ..Default::default()
            }),
        );

        // Git diff scopes
        t.insert("diff.plus", style::new().fg(sage));
        t.insert("diff.minus", style::new().fg(cinnabar));
        t.insert("diff.delta", style::new().fg(amber));

        // LSP diagnostic scopes
        t.insert(
            "diagnostic.error",
            style::new().fg(cinnabar).modifier(modifier {
                bold: true,
                ..Default::default()
            }),
        );
        t.insert("diagnostic.warning", style::new().fg(amber));
        t.insert("diagnostic.info", style::new().fg(cerulean));
        t.insert("diagnostic.hint", style::new().fg(verdigris));

        // Syntax highlighting scopes
        t.insert(
            "comment",
            style::new().fg(moss).modifier(modifier {
                italic: true,
                ..Default::default()
            }),
        );
        t.insert(
            "comment.doc",
            style::new().fg(lichen).modifier(modifier {
                italic: true,
                ..Default::default()
            }),
        );
        t.insert(
            "keyword",
            style::new().fg(terracotta).modifier(modifier {
                bold: true,
                ..Default::default()
            }),
        );
        t.insert("function", style::new().fg(cerulean));
        t.insert("string", style::new().fg(sage));
        t.insert("constant", style::new().fg(apricot));
        t.insert("type", style::new().fg(amber));
        t.insert("variable", style::new().fg(parchment));
        t.insert("operator", style::new().fg(heather));
        t.insert("punctuation", style::new().fg(sandstone));

        t
    }

    /// Default dark theme fallback (aliases to `default_dacctal`).
    #[inline]
    #[must_use]
    pub fn default_dark() -> Self {
        Self::default_dacctal()
    }
}

/// Recursively loads a theme TOML value, resolving `inherits = "parent"` references.
fn load_theme_value(name: &str, visited: &mut HashSet<String>) -> Result<Value, theme_error> {
    if visited.contains(name) {
        return Err(theme_error::cycle_detected(name.to_string()));
    }
    visited.insert(name.to_string());

    let file_path = find_theme_file(name)
        .ok_or_else(|| theme_error::io_error(format!("theme file not found for '{name}'")))?;

    let contents = fs::read_to_string(&file_path).map_err(|e| {
        theme_error::io_error(format!("failed to read {}: {e}", file_path.display()))
    })?;

    let theme_val: Value = toml::from_str(&contents).map_err(|e| {
        theme_error::parse_error(format!("failed to parse {}: {e}", file_path.display()))
    })?;

    if let Some(parent_val) = theme_val.get("inherits") {
        if let Some(parent_name) = parent_val.as_str() {
            let parent_theme_val = if parent_name == "default"
                || parent_name == "dacctal"
                || parent_name == "ned_dark"
            {
                Value::Table(toml::map::Map::new())
            } else {
                load_theme_value(parent_name, visited)?
            };
            return Ok(merge_toml(parent_theme_val, theme_val, 2));
        }
    }

    Ok(theme_val)
}

/// Parses a resolved TOML value into a [`theme`] object.
fn parse_theme_from_value(name: &str, value: Value) -> Result<theme, theme_error> {
    let mut t = theme::new(name);
    let table = match value {
        Value::Table(map) => map,
        _ => return Ok(t),
    };

    // Parse palette table if present
    let mut palette: HashMap<String, String> = HashMap::new();
    if let Some(Value::Table(pal_table)) = table.get("palette") {
        for (k, v) in pal_table {
            if let Some(color_str) = v.as_str() {
                palette.insert(k.clone(), color_str.to_string());
            }
        }
    }

    // Parse each scope
    for (k, v) in table {
        if k == "palette" || k == "inherits" {
            continue;
        }
        let parsed_style = parse_style_entry(&v, &palette);
        t.insert(k, parsed_style);
    }

    Ok(t)
}

/// Resolves a single style entry from TOML, translating palette variables.
fn parse_style_entry(entry: &Value, palette: &HashMap<String, String>) -> style {
    let mut s = style::new();

    let resolve_color = |name: &str| -> Option<color> {
        let actual_color = palette.get(name).map_or(name, std::string::String::as_str);
        color::parse(actual_color)
    };

    match entry {
        Value::String(simple_fg) => {
            if let Some(c) = resolve_color(simple_fg) {
                s.fg = Some(c);
            }
        }
        Value::Table(tbl) => {
            if let Some(fg_val) = tbl.get("fg").and_then(Value::as_str) {
                s.fg = resolve_color(fg_val);
            }
            if let Some(bg_val) = tbl.get("bg").and_then(Value::as_str) {
                s.bg = resolve_color(bg_val);
            }
            if let Some(Value::Array(mods)) = tbl.get("modifiers") {
                let mut m = modifier::default();
                for m_val in mods {
                    if let Some(m_str) = m_val.as_str() {
                        match m_str {
                            "bold" => m.bold = true,
                            "dim" => m.dim = true,
                            "italic" => m.italic = true,
                            "underlined" | "underline" => m.underline = true,
                            "crossed_out" | "strikethrough" => m.crossed_out = true,
                            _ => {}
                        }
                    }
                }
                s.modifier = m;
            }
        }
        _ => {}
    }

    s
}
