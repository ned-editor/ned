//! Cursor line styling and cursor coordinate positioning matching Helix.

use ned_core::documents::document;
use ned_core::views::view;
use ned_tui::output::style::Style;

use crate::theme_bridge::merge_style;
use crate::ui::cursor_render_info;

/// Resolves the background style for an editor row, applying cursorline highlighting if active.
#[must_use]
pub fn resolve_line_background(
    is_active_line: bool,
    cursorline_enabled: bool,
    default_style: Style,
    cursorline_style: Style,
) -> Style {
    if is_active_line && cursorline_enabled {
        merge_style(default_style, cursorline_style)
    } else {
        default_style
    }
}

/// Computes the terminal cursor coordinates and shape for the active view and document.
#[must_use]
pub fn compute_cursor_info(
    view: &view,
    doc: &document,
    gutter_width: u16,
    current_mode: ned_config::mode,
    shape_config: &ned_config::editor::cursor_shape_config,
) -> cursor_render_info {
    let (cursor_col, cursor_row, visible) = if let Some((c_col, c_row)) = view.cursor_coords(doc) {
        (gutter_width + c_col, c_row, true)
    } else {
        (gutter_width, 0, false)
    };

    let shape = match current_mode {
        ned_config::mode::normal => shape_config.normal,
        ned_config::mode::insert => shape_config.insert,
        ned_config::mode::select => shape_config.select,
    };

    cursor_render_info {
        col: cursor_col,
        row: cursor_row,
        shape,
        visible,
    }
}
