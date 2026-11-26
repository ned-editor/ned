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
    /// Falls back to the built-in dark theme if `name` is `"default"` or if the file cannot be located.
    #[must_use]
    pub fn load_or_default(name: &str) -> Self {
        if name == "default" || name == "ned_dark" {
            return Self::default_dark();
        }

        Self::load(name).unwrap_or_else(|_| Self::default_dark())
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

    /// Built-in modern dark theme providing complete coverage for UI, Git diffs, and diagnostics.
    #[must_use]
    pub fn default_dark() -> Self {
        let mut t = Self::new("ned_dark");

        let bg = color::rgb(0x1e, 0x1e, 0x2e); // Catppuccin Base
        let fg = color::rgb(0xcd, 0xd6, 0xf4); // Catppuccin Text
        let mantle = color::rgb(0x18, 0x18, 0x25);
        let surface0 = color::rgb(0x31, 0x32, 0x44);
        let surface1 = color::rgb(0x45, 0x47, 0x5a);
        let overlay0 = color::rgb(0x6c, 0x70, 0x86);

        let c_red = color::rgb(0xf3, 0x8b, 0xa8);
        let c_green = color::rgb(0xa6, 0xe3, 0xa1);
        let c_yellow = color::rgb(0xf9, 0xe2, 0xaf);
        let c_blue = color::rgb(0x89, 0xb4, 0xfa);
        let c_mauve = color::rgb(0xcb, 0xa6, 0xf7);
        let c_cyan = color::rgb(0x94, 0xe2, 0xd5);

        // UI scopes
        t.insert("ui.background", style::new().bg(bg));
        t.insert("ui.text", style::new().fg(fg));
        t.insert("ui.text.focus", style::new().fg(fg).modifier(modifier { bold: true, ..Default::default() }));
        t.insert("ui.cursor", style::new().fg(bg).bg(fg));
        t.insert("ui.cursor.normal", style::new().fg(bg).bg(fg));
        t.insert("ui.cursor.insert", style::new().fg(bg).bg(c_blue));
        t.insert("ui.cursor.select", style::new().fg(bg).bg(c_green));
        t.insert("ui.cursor.primary", style::new().fg(bg).bg(fg).modifier(modifier { bold: true, ..Default::default() }));
        t.insert("ui.selection", style::new().bg(surface0));
        t.insert("ui.selection.primary", style::new().bg(surface1));
        t.insert("ui.linenr", style::new().fg(overlay0));
        t.insert("ui.linenr.selected", style::new().fg(fg).modifier(modifier { bold: true, ..Default::default() }));
        t.insert("ui.statusline", style::new().fg(fg).bg(mantle));
        t.insert("ui.statusline.inactive", style::new().fg(overlay0).bg(mantle));
        t.insert("ui.gutter", style::new().bg(bg));
        t.insert("ui.popup", style::new().fg(fg).bg(mantle));
        t.insert("ui.menu", style::new().fg(fg).bg(mantle));
        t.insert("ui.menu.selected", style::new().fg(bg).bg(c_blue));

        // Git diff scopes
        t.insert("diff.plus", style::new().fg(c_green));
        t.insert("diff.minus", style::new().fg(c_red));
        t.insert("diff.delta", style::new().fg(c_yellow));

        // LSP diagnostic scopes
        t.insert("diagnostic.error", style::new().fg(c_red).modifier(modifier { bold: true, ..Default::default() }));
        t.insert("diagnostic.warning", style::new().fg(c_yellow));
        t.insert("diagnostic.info", style::new().fg(c_blue));
        t.insert("diagnostic.hint", style::new().fg(c_cyan));

        // Basic syntax highlighting scopes
        t.insert("comment", style::new().fg(overlay0).modifier(modifier { italic: true, ..Default::default() }));
        t.insert("keyword", style::new().fg(c_mauve).modifier(modifier { bold: true, ..Default::default() }));
        t.insert("function", style::new().fg(c_blue));
        t.insert("string", style::new().fg(c_green));
        t.insert("constant", style::new().fg(c_yellow));
        t.insert("type", style::new().fg(c_yellow));
        t.insert("variable", style::new().fg(fg));

        t
    }
}

/// Recursively loads a theme TOML value, resolving `inherits = "parent"` references.
fn load_theme_value(name: &str, visited: &mut HashSet<String>) -> Result<Value, theme_error> {
    if visited.contains(name) {
        return Err(theme_error::cycle_detected(name.to_string()));
    }
    visited.insert(name.to_string());

    let file_path = find_theme_file(name).ok_or_else(|| {
        theme_error::io_error(format!("theme file not found for '{name}'"))
    })?;

    let contents = fs::read_to_string(&file_path)
        .map_err(|e| theme_error::io_error(format!("failed to read {}: {e}", file_path.display())))?;

    let theme_val: Value = toml::from_str(&contents)
        .map_err(|e| theme_error::parse_error(format!("failed to parse {}: {e}", file_path.display())))?;

    if let Some(parent_val) = theme_val.get("inherits") {
        if let Some(parent_name) = parent_val.as_str() {
            let parent_theme_val = if parent_name == "default" || parent_name == "ned_dark" {
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
        let actual_color = palette.get(name).map(|s| s.as_str()).unwrap_or(name);
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
