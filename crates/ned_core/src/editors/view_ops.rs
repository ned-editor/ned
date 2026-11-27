//! View lifecycle, cursor tracking, mouse handling, and scrolling operations.

use ::ned_config::mode;
use ned_text::selection;

use crate::documents::document_id;
use crate::editors::editor;
use crate::views::{view, view_id};

/// Creates a new view displaying `doc_id` with specified dimensions.
pub fn create_view(ed: &mut editor, doc_id: document_id, width: u16, height: u16) -> view_id {
    let vid = ed.next_view_id;
    ed.next_view_id += 1;
    ed.views.insert(vid, view::new(vid, doc_id, width, height));
    vid
}

/// Returns a reference to the active view.
#[inline]
#[must_use]
pub fn active_view(ed: &editor) -> &view {
    ed.views
        .get(&ed.active_view_id)
        .expect("active view must exist")
}

/// Returns a mutable reference to the active view.
#[inline]
pub fn active_view_mut(ed: &mut editor) -> &mut view {
    ed.views
        .get_mut(&ed.active_view_id)
        .expect("active view must exist")
}

/// Ensures the active cursor is within the viewport margins.
pub fn ensure_active_cursor_visible(ed: &mut editor) {
    let scrolloff = ed.config.editor.scrolloff;
    let doc_id = ed.active_doc_id;
    let view_id = ed.active_view_id;
    if let (Some(doc), Some(view)) = (ed.documents.get(&doc_id), ed.views.get_mut(&view_id)) {
        view.ensure_cursor_visible(doc, scrolloff);
    }
}

/// Handles a mouse click event at screen coordinates `(col, row)`.
pub fn handle_mouse_click(
    ed: &mut editor,
    col: u16,
    row: u16,
    gutter_width: u16,
    editor_height: u16,
    extend: bool,
) {
    ed.hover_popup = None;
    if row >= editor_height {
        return;
    }

    let doc_id = ed.active_doc_id;
    let view_id = ed.active_view_id;

    let (target_pos, scrolloff) = {
        let doc = ed.documents.get(&doc_id).expect("active doc must exist");
        let view = ed.views.get(&view_id).expect("active view must exist");
        let text_col = col.saturating_sub(gutter_width);
        let pos = view.pos_at_coords(doc, text_col, row);
        (pos, ed.config.editor.scrolloff)
    };

    let doc = ed.documents.get_mut(&doc_id).expect("active doc must exist");
    if extend || ed.mode == mode::select {
        let primary = doc.selection().primary();
        let new_range = primary.put_cursor(doc.text(), target_pos, true);
        doc.set_selection(selection::single(new_range.anchor, new_range.head));
    } else {
        doc.set_selection(selection::point(target_pos));
    }

    let view = ed.views.get_mut(&view_id).expect("active view must exist");
    view.ensure_cursor_visible(doc, scrolloff);
}

/// Scrolls the active view vertically by `delta` lines matching Helix scroll behavior.
///
/// Preserves primary cursor position unless scrolled past the viewport margins (scrolloff).
pub fn scroll_view_by(ed: &mut editor, delta: isize) {
    let doc_id = ed.active_doc_id;
    let view_id = ed.active_view_id;

    let total_lines = ed
        .documents
        .get(&doc_id)
        .map_or(1, |d| d.text().line_count().max(1));

    let view = match ed.views.get_mut(&view_id) {
        Some(v) => v,
        None => return,
    };

    let height = view.height as usize;
    if height == 0 {
        return;
    }

    let scrolloff = ed.config.editor.scrolloff.min(height.saturating_sub(1) / 2);
    let max_row_offset = total_lines.saturating_sub(1);

    if delta > 0 {
        let offset = delta as usize;
        view.row_offset = (view.row_offset + offset).min(max_row_offset);
    } else if delta < 0 {
        let offset = delta.unsigned_abs();
        view.row_offset = view.row_offset.saturating_sub(offset);
    }

    let new_row_offset = view.row_offset;

    // Keep cursor within [new_row_offset + scrolloff, new_row_offset + height - scrolloff - 1]
    let min_cursor_line = (new_row_offset + scrolloff).min(total_lines.saturating_sub(1));
    let max_cursor_line = (new_row_offset + height)
        .saturating_sub(scrolloff + 1)
        .min(total_lines.saturating_sub(1));

    let doc = ed.documents.get_mut(&doc_id).expect("active doc must exist");
    let buffer = doc.text();
    let primary_cursor = doc.selection().primary().cursor(buffer);
    let cursor_byte = buffer.char_to_byte(primary_cursor);
    let cursor_line = buffer.byte_to_line(cursor_byte);
    let line_start_byte = buffer.line_to_byte(cursor_line).unwrap_or(0);
    let line_start_char = buffer.byte_to_char(line_start_byte);
    let cursor_col = primary_cursor.saturating_sub(line_start_char);

    let target_line = if cursor_line < min_cursor_line {
        Some(min_cursor_line)
    } else if cursor_line > max_cursor_line {
        Some(max_cursor_line)
    } else {
        None
    };

    if let Some(new_line) = target_line {
        let target_line_byte = buffer.line_to_byte(new_line).unwrap_or(0);
        let target_line_char = buffer.byte_to_char(target_line_byte);
        let target_line_len = if let Some(line_str) = buffer.line(new_line) {
            line_str.trim_end_matches(['\r', '\n']).chars().count()
        } else {
            0
        };
        let target_col = cursor_col.min(target_line_len);
        let target_pos = target_line_char + target_col;

        if ed.mode == mode::select {
            let primary = doc.selection().primary();
            doc.set_selection(selection::single(primary.anchor, target_pos));
        } else {
            doc.set_selection(selection::point(target_pos));
        }
    }
}
