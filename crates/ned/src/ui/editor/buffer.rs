//! Text buffer background, syntax token styling, and beyond-EOF virtual line painting.

use ned_config::theme;
use ned_tree::highlight_span;
use ned_tui::geometry::point;
use ned_tui::output::style::Style;
use ned_tui::Screen;

use crate::theme_bridge::{merge_style, style_for_scope};

/// Fills the text area background for a single row.
pub fn fill_line_background(
    screen: &mut Screen,
    row: u16,
    gutter_width: u16,
    width: u16,
    line_bg: &Style,
) {
    for c in gutter_width..width {
        screen.set(point(c, row), line_bg, " ");
    }
}

/// Resolves the base syntax highlight style for a character in a line against active spans.
#[must_use]
pub fn resolve_syntax_style(
    th: &theme,
    line_spans: &[highlight_span],
    char_idx_in_line: usize,
    base_style: Style,
) -> Style {
    if let Some(span) = line_spans
        .iter()
        .find(|s| char_idx_in_line >= s.start && char_idx_in_line < s.end)
    {
        let scope_style = style_for_scope(th, span.scope());
        merge_style(base_style, scope_style)
    } else {
        base_style
    }
}

/// Paints empty space beyond EOF using virtual line indicator `~` matching Helix layout.
pub fn render_beyond_eof(
    screen: &mut Screen,
    row: u16,
    gutter_width: u16,
    width: u16,
    gutter_style: &Style,
    default_style: &Style,
) {
    screen.set(point(0, row), gutter_style, " ");
    screen.set(point(1, row), gutter_style, "~");
    for c in 2..gutter_width {
        screen.set(point(c, row), gutter_style, " ");
    }
    for c in gutter_width..width {
        screen.set(point(c, row), default_style, " ");
    }
}
