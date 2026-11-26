//! Terminal color and text style representation for themes and editor rendering.

/// Represents an ANSI, 256-color, or 24-bit true color value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum color {
    /// Resets color to default terminal foreground/background.
    reset,
    /// Standard black.
    black,
    /// Standard red.
    red,
    /// Standard green.
    green,
    /// Standard yellow.
    yellow,
    /// Standard blue.
    blue,
    /// Standard magenta.
    magenta,
    /// Standard cyan.
    cyan,
    /// Standard white.
    white,
    /// High-intensity bright black (often gray).
    bright_black,
    /// High-intensity bright red.
    bright_red,
    /// High-intensity bright green.
    bright_green,
    /// High-intensity bright yellow.
    bright_yellow,
    /// High-intensity bright blue.
    bright_blue,
    /// High-intensity bright magenta.
    bright_magenta,
    /// High-intensity bright cyan.
    bright_cyan,
    /// High-intensity bright white.
    bright_white,
    /// Fixed 8-bit ANSI palette index (0–255).
    ansi(u8),
    /// 24-bit RGB true color.
    rgb(u8, u8, u8),
}

impl color {
    /// Parses a color string from hex (`"#1e1e2e"`, `"#fff"`), named strings, or ANSI indices.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.starts_with('#') {
            return Self::from_hex(trimmed);
        }

        if let Ok(num) = trimmed.parse::<u8>() {
            return Some(Self::ansi(num));
        }

        Self::from_name(trimmed)
    }

    /// Parses a 6-digit or 3-digit hexadecimal color string.
    #[must_use]
    pub fn from_hex(hex: &str) -> Option<Self> {
        let stripped = hex.strip_prefix('#')?;
        match stripped.len() {
            6 => {
                let r = u8::from_str_radix(&stripped[0..2], 16).ok()?;
                let g = u8::from_str_radix(&stripped[2..4], 16).ok()?;
                let b = u8::from_str_radix(&stripped[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            3 => {
                let r = u8::from_str_radix(&stripped[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&stripped[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&stripped[2..3], 16).ok()? * 17;
                Some(Self::rgb(r, g, b))
            }
            _ => None,
        }
    }

    /// Parses standard ANSI color names.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "reset" | "default" => Some(Self::reset),
            "black" => Some(Self::black),
            "red" => Some(Self::red),
            "green" => Some(Self::green),
            "yellow" => Some(Self::yellow),
            "blue" => Some(Self::blue),
            "magenta" => Some(Self::magenta),
            "cyan" => Some(Self::cyan),
            "white" => Some(Self::white),
            "light-black" | "bright-black" | "gray" | "grey" => Some(Self::bright_black),
            "light-red" | "bright-red" => Some(Self::bright_red),
            "light-green" | "bright-green" => Some(Self::bright_green),
            "light-yellow" | "bright-yellow" => Some(Self::bright_yellow),
            "light-blue" | "bright-blue" => Some(Self::bright_blue),
            "light-magenta" | "bright-magenta" => Some(Self::bright_magenta),
            "light-cyan" | "bright-cyan" => Some(Self::bright_cyan),
            "light-white" | "bright-white" => Some(Self::bright_white),
            _ => None,
        }
    }
}

/// Text formatting modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct modifier {
    /// Bold text weight.
    pub bold: bool,
    /// Dim or faint text weight.
    pub dim: bool,
    /// Italic text style.
    pub italic: bool,
    /// Underlined text style.
    pub underline: bool,
    /// Crossed-out or strikethrough text style.
    pub crossed_out: bool,
}

impl modifier {
    /// Combines two sets of modifiers with logical OR.
    #[must_use]
    pub fn combine(self, other: Self) -> Self {
        Self {
            bold: self.bold || other.bold,
            dim: self.dim || other.dim,
            italic: self.italic || other.italic,
            underline: self.underline || other.underline,
            crossed_out: self.crossed_out || other.crossed_out,
        }
    }
}

/// Complete text style consisting of foreground, background, and formatting modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct style {
    /// Foreground text color.
    pub fg: Option<color>,
    /// Background surface color.
    pub bg: Option<color>,
    /// Text styling modifiers.
    pub modifier: modifier,
}

impl style {
    /// Creates an empty style.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            modifier: modifier {
                bold: false,
                dim: false,
                italic: false,
                underline: false,
                crossed_out: false,
            },
        }
    }

    /// Sets the foreground color.
    #[must_use]
    pub const fn fg(mut self, c: color) -> Self {
        self.fg = Some(c);
        self
    }

    /// Sets the background color.
    #[must_use]
    pub const fn bg(mut self, c: color) -> Self {
        self.bg = Some(c);
        self
    }

    /// Sets the text modifiers.
    #[must_use]
    pub const fn modifier(mut self, m: modifier) -> Self {
        self.modifier = m;
        self
    }

    /// Patches this style with another style, overriding non-empty fields.
    #[must_use]
    pub fn patch(self, other: Self) -> Self {
        Self {
            fg: other.fg.or(self.fg),
            bg: other.bg.or(self.bg),
            modifier: self.modifier.combine(other.modifier),
        }
    }
}
