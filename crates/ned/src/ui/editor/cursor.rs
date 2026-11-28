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
    tab_width: usize,
    current_mode: ned_config::mode,
    shape_config: &ned_config::editor::cursor_shape_config,
) -> cursor_render_info {
    let buffer = doc.text();
    let primary_cursor = doc.selection().primary().cursor(buffer);
    let cursor_byte = buffer.char_to_byte(primary_cursor);
    let cursor_line = buffer.byte_to_line(cursor_byte);

    let (cursor_col, cursor_row, visible) = if cursor_line >= view.row_offset
        && cursor_line < view.row_offset + (view.height as usize)
    {
        let line_str = buffer.line(cursor_line).unwrap_or_default();
        let clean_str = line_str.trim_end_matches(['\r', '\n']);
        let line_start_byte = buffer.line_to_byte(cursor_line).unwrap_or(0);
        let line_start_char = buffer.byte_to_char(line_start_byte);
        let cursor_char_in_line = primary_cursor.saturating_sub(line_start_char);

        let tab_size = tab_width.max(1);
        let mut visual_col = 0;
        let mut curr_char = 0;
        for g in ned_tui::unicode::str_graphemes(clean_str) {
            if curr_char >= cursor_char_in_line {
                break;
            }
            let g_width = if g == "\t" {
                tab_size.saturating_sub(visual_col % tab_size).max(1)
            } else {
                ned_tui::unicode::grapheme_width(g)
            };
            visual_col += g_width;
            curr_char += g.chars().count();
        }

        if visual_col >= view.col_offset && visual_col < view.col_offset + (view.width as usize) {
            let col = gutter_width + (visual_col - view.col_offset) as u16;
            let row = (cursor_line - view.row_offset) as u16;
            (col, row, true)
        } else {
            (gutter_width, 0, false)
        }
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
