//! colors, modifiers, and text styles with ansi escape sequence formatting.

use std::fmt::Write;

/// representation of a terminal foreground or background color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum color {
    /// default terminal color.
    #[default]
    reset,
    /// standard ansi black.
    black,
    /// standard ansi red.
    red,
    /// standard ansi green.
    green,
    /// standard ansi yellow.
    yellow,
    /// standard ansi blue.
    blue,
    /// standard ansi magenta.
    magenta,
    /// standard ansi cyan.
    cyan,
    /// standard ansi white.
    white,
    /// 256-color indexed value.
    indexed(u8),
    /// 24-bit truecolor rgb value.
    rgb(u8, u8, u8),
}

/// text attributes and formatting modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct modifier {
    /// bold or increased intensity.
    pub bold: bool,
    /// dim or decreased intensity.
    pub dim: bool,
    /// italic text.
    pub italic: bool,
    /// single underline.
    pub underline: bool,
    /// reverse video (swap foreground and background).
    pub reverse: bool,
}

/// styling specification containing foreground color, background color, and modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct style {
    /// foreground text color.
    pub fg: color,
    /// background fill color.
    pub bg: color,
    /// active modifiers.
    pub modifier: modifier,
}

impl style {
    /// creates a style with default colors and no modifiers.
    #[must_use]
    pub fn reset() -> Self {
        Self::default()
    }

    /// sets the foreground color.
    #[must_use]
    pub fn fg(mut self, c: color) -> Self {
        self.fg = c;
        self
    }

    /// sets the background color.
    #[must_use]
    pub fn bg(mut self, c: color) -> Self {
        self.bg = c;
        self
    }

    /// enables bold modifier.
    #[must_use]
    pub fn bold(mut self) -> Self {
        self.modifier.bold = true;
        self
    }

    /// enables italic modifier.
    #[must_use]
    pub fn italic(mut self) -> Self {
        self.modifier.italic = true;
        self
    }

    /// enables underline modifier.
    #[must_use]
    pub fn underline(mut self) -> Self {
        self.modifier.underline = true;
        self
    }

    /// enables reverse modifier.
    #[must_use]
    pub fn reverse(mut self) -> Self {
        self.modifier.reverse = true;
        self
    }

    /// appends ansi escape sequences to transition from `previous` style to `self`.
    pub fn write_diff_ansi(&self, previous: &style, out: &mut String) {
        if self == previous {
            return;
        }

        // if modifiers were turned off, issue full reset and re-apply
        let turned_off_modifier = (previous.modifier.bold && !self.modifier.bold)
            || (previous.modifier.dim && !self.modifier.dim)
            || (previous.modifier.italic && !self.modifier.italic)
            || (previous.modifier.underline && !self.modifier.underline)
            || (previous.modifier.reverse && !self.modifier.reverse);

        if turned_off_modifier {
            out.push_str("\x1b[0m");
            self.write_all_ansi(out);
            return;
        }

        // apply new modifiers
        if self.modifier.bold && !previous.modifier.bold {
            out.push_str("\x1b[1m");
        }
        if self.modifier.dim && !previous.modifier.dim {
            out.push_str("\x1b[2m");
        }
        if self.modifier.italic && !previous.modifier.italic {
            out.push_str("\x1b[3m");
        }
        if self.modifier.underline && !previous.modifier.underline {
            out.push_str("\x1b[4m");
        }
        if self.modifier.reverse && !previous.modifier.reverse {
            out.push_str("\x1b[7m");
        }

        // apply foreground color change
        if self.fg != previous.fg {
            write_color(self.fg, true, out);
        }

        // apply background color change
        if self.bg != previous.bg {
            write_color(self.bg, false, out);
        }
    }

    /// writes full ansi sequence establishing this style.
    pub fn write_all_ansi(&self, out: &mut String) {
        if self.modifier.bold {
            out.push_str("\x1b[1m");
        }
        if self.modifier.dim {
            out.push_str("\x1b[2m");
        }
        if self.modifier.italic {
            out.push_str("\x1b[3m");
        }
        if self.modifier.underline {
            out.push_str("\x1b[4m");
        }
        if self.modifier.reverse {
            out.push_str("\x1b[7m");
        }

        if self.fg != color::reset {
            write_color(self.fg, true, out);
        }
        if self.bg != color::reset {
            write_color(self.bg, false, out);
        }
    }
}

fn write_color(c: color, is_fg: bool, out: &mut String) {
    let (base_code, ext_code) = if is_fg { (30, 38) } else { (40, 48) };
    match c {
        color::reset => {
            let code = if is_fg { 39 } else { 49 };
            let _ = write!(out, "\x1b[{code}m");
        }
        color::black => {
            let _ = write!(out, "\x1b[{}m", base_code);
        }
        color::red => {
            let _ = write!(out, "\x1b[{}m", base_code + 1);
        }
        color::green => {
            let _ = write!(out, "\x1b[{}m", base_code + 2);
        }
        color::yellow => {
            let _ = write!(out, "\x1b[{}m", base_code + 3);
        }
        color::blue => {
            let _ = write!(out, "\x1b[{}m", base_code + 4);
        }
        color::magenta => {
            let _ = write!(out, "\x1b[{}m", base_code + 5);
        }
        color::cyan => {
            let _ = write!(out, "\x1b[{}m", base_code + 6);
        }
        color::white => {
            let _ = write!(out, "\x1b[{}m", base_code + 7);
        }
        color::indexed(idx) => {
            let _ = write!(out, "\x1b[{ext_code};5;{idx}m");
        }
        color::rgb(r, g, b) => {
            let _ = write!(out, "\x1b[{ext_code};2;{r};{g};{b}m");
        }
    }
}
