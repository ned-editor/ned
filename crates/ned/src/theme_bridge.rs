//! Bridge converting `ned_config` theme styles into `ned_tui` rendering styles.

use ned_config::{color as config_color, style as config_style, theme};
use ned_tui::output::color::Color;
use ned_tui::output::style::Style;

/// Converts a `ned_config` color into an optional `ned_tui` color.
#[must_use]
pub fn to_tui_color(c: config_color) -> Option<Color> {
    match c {
        config_color::reset => None,
        config_color::black => Some(Color::black()),
        config_color::red => Some(Color::red(false)),
        config_color::green => Some(Color::green(false)),
        config_color::yellow => Some(Color::yellow(false)),
        config_color::blue => Some(Color::blue(false)),
        config_color::magenta => Some(Color::magenta(false)),
        config_color::cyan => Some(Color::cyan(false)),
        config_color::white => Some(Color::white()),
        config_color::bright_black => Some(Color::grey(false)),
        config_color::bright_red => Some(Color::red(true)),
        config_color::bright_green => Some(Color::green(true)),
        config_color::bright_yellow => Some(Color::yellow(true)),
        config_color::bright_blue => Some(Color::blue(true)),
        config_color::bright_magenta => Some(Color::magenta(true)),
        config_color::bright_cyan => Some(Color::cyan(true)),
        config_color::bright_white => Some(Color::grey(true)),
        config_color::ansi(idx) => Some(Color::ansi(idx)),
        config_color::rgb(r, g, b) => Some(Color::rgb(r, g, b)),
    }
}

/// Converts a `ned_config` style into a `ned_tui` style.
#[must_use]
pub fn to_tui_style(s: config_style) -> Style {
    Style {
        front_color: s.fg.and_then(to_tui_color),
        back_color: s.bg.and_then(to_tui_color),
        frame_color: None,
        bold: s.modifier.bold.then_some(true),
        italic: s.modifier.italic.then_some(true),
        underlined: s.modifier.underline.then_some(true),
        crossed_out: s.modifier.crossed_out.then_some(true),
        blink: None,
    }
}

/// Merges an overlay style onto a base style with Helix patch semantics.
#[must_use]
pub fn merge_style(base: Style, overlay: Style) -> Style {
    Style {
        front_color: overlay.front_color.or(base.front_color),
        back_color: overlay.back_color.or(base.back_color),
        frame_color: overlay.frame_color.or(base.frame_color),
        bold: overlay.bold.or(base.bold),
        italic: overlay.italic.or(base.italic),
        underlined: overlay.underlined.or(base.underlined),
        crossed_out: overlay.crossed_out.or(base.crossed_out),
        blink: overlay.blink.or(base.blink),
    }
}

/// Resolves a theme scope string into a `ned_tui` style.
#[must_use]
pub fn style_for_scope(active_theme: &theme, scope: &str) -> Style {
    to_tui_style(active_theme.get(scope))
}
