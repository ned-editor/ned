//! View geometry, viewport scrolling, and screen cursor coordinate calculation.

use std::ops::Range;

use crate::documents::{document, document_id};

/// Unique identifier for a viewport view.
pub type view_id = usize;

/// Viewport displaying a slice of a document on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct view {
    /// Unique view identifier.
    pub id: view_id,
    /// Associated document identifier.
    pub doc_id: document_id,
    /// First visible line row offset in the document.
    pub row_offset: usize,
    /// Horizontal character column scroll offset.
    pub col_offset: usize,
    /// Visible width in terminal columns.
    pub width: u16,
    /// Visible height in terminal rows.
    pub height: u16,
}

impl view {
    /// Creates a new view displaying `doc_id` with initial dimensions.
    #[must_use]
    pub fn new(id: view_id, doc_id: document_id, width: u16, height: u16) -> Self {
        Self {
            id,
            doc_id,
            row_offset: 0,
            col_offset: 0,
            width,
            height,
        }
    }

    /// Resizes the viewport dimensions.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    /// Adjusts viewport row and column offsets to guarantee the primary cursor is visible,
    /// respecting the configured `scrolloff` margin.
    pub fn ensure_cursor_visible(&mut self, doc: &document, scrolloff: usize) {
        if self.height == 0 || self.width == 0 {
            return;
        }

        let buffer = doc.text();
        let primary_cursor = doc.selection().primary().cursor(buffer);
        let cursor_byte = buffer.char_to_byte(primary_cursor);
        let cursor_line = buffer.byte_to_line(cursor_byte);
        let line_start_byte = buffer.line_to_byte(cursor_line).unwrap_or(0);
        let line_start_char = buffer.byte_to_char(line_start_byte);
        let cursor_col = primary_cursor.saturating_sub(line_start_char);

        let height = self.height as usize;
        let effective_scrolloff = scrolloff.min(height / 2);

        // Vertical scrolling
        if cursor_line < self.row_offset + effective_scrolloff {
            self.row_offset = cursor_line.saturating_sub(effective_scrolloff);
        } else if cursor_line + effective_scrolloff >= self.row_offset + height {
            self.row_offset = (cursor_line + effective_scrolloff + 1).saturating_sub(height);
        }

        // Horizontal scrolling
        let width = self.width as usize;
        if cursor_col < self.col_offset {
            self.col_offset = cursor_col;
        } else if cursor_col >= self.col_offset + width {
            self.col_offset = (cursor_col + 1).saturating_sub(width);
        }
    }

    /// Returns the range of line indices currently visible in this view.
    #[must_use]
    pub fn visible_lines(&self) -> Range<usize> {
        let start = self.row_offset;
        let end = start + (self.height as usize);
        start..end
    }

    /// Computes the relative terminal screen coordinates `(col, row)` for the primary cursor,
    /// or `None` if the cursor lies outside the visible viewport.
    #[must_use]
    pub fn cursor_coords(&self, doc: &document) -> Option<(u16, u16)> {
        let buffer = doc.text();
        let primary_cursor = doc.selection().primary().cursor(buffer);
        let cursor_byte = buffer.char_to_byte(primary_cursor);
        let cursor_line = buffer.byte_to_line(cursor_byte);
        let line_start_byte = buffer.line_to_byte(cursor_line).unwrap_or(0);
        let line_start_char = buffer.byte_to_char(line_start_byte);
        let cursor_col = primary_cursor.saturating_sub(line_start_char);

        if cursor_line < self.row_offset || cursor_line >= self.row_offset + (self.height as usize) {
            return None;
        }
        if cursor_col < self.col_offset || cursor_col >= self.col_offset + (self.width as usize) {
            return None;
        }

        let row = (cursor_line - self.row_offset) as u16;
        let col = (cursor_col - self.col_offset) as u16;
        Some((col, row))
    }
}
