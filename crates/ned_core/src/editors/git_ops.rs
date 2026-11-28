//! Git operations: diff hunk navigation, diff refresh helpers, and repository status.

use ned_text::selection;

use crate::documents::document_id;
use crate::editors::editor;

/// Maximum file byte size allowed for diff computation (1 MiB).
pub const max_diff_bytes: usize = 1024 * 1024;

/// Maximum line count allowed for diff computation (20,000 lines).
pub const max_diff_lines: usize = 20_000;

/// Refreshes the diff hunks for the specified document and updates editor status if capped.
pub fn refresh_diff(ed: &mut editor, doc_id: document_id) {
    if let Some(doc) = ed.documents.get_mut(&doc_id) {
        if let Err(msg) = doc.reload_diff() {
            ed.status_message = Some(msg.to_string());
        }
    }
}

/// Jumps cursor to the start of the next diff hunk in the active document.
///
/// # Panics
///
/// Panics if the current cursor line number exceeds `u32::MAX`.
pub fn goto_next_hunk(ed: &mut editor) {
    let doc_id = ed.active_doc_id;
    let target_line = {
        let Some(doc) = ed.documents.get(&doc_id) else {
            return;
        };

        if doc.is_diff_too_large() {
            ed.status_message = Some("diff too large".to_string());
            return;
        }

        let hunks = doc.diff_hunks();
        if hunks.is_empty() {
            ed.status_message = Some("no diff hunks".to_string());
            return;
        }

        let cursor_char = doc.selection().primary().cursor(doc.text());
        let cursor_byte = doc.text().char_to_byte(cursor_char);
        let current_line = u32::try_from(doc.text().byte_to_line(cursor_byte))
            .expect("line number fits in u32");

        let target_hunk = hunks
            .iter()
            .find(|h| h.after_start > current_line)
            .or_else(|| hunks.first());

        match target_hunk {
            Some(h) => {
                let max_line = doc.text().line_count().saturating_sub(1);
                (h.after_start as usize).min(max_line)
            }
            None => return,
        }
    };

    let doc = ed.active_doc_mut();
    let target_byte = doc.text().line_to_byte(target_line).unwrap_or(0);
    let target_char = doc.text().byte_to_char(target_byte);
    doc.set_selection(selection::point(target_char));
    ed.ensure_active_cursor_visible();
}

/// Jumps cursor to the start of the previous diff hunk in the active document.
///
/// # Panics
///
/// Panics if the current cursor line number exceeds `u32::MAX`.
pub fn goto_prev_hunk(ed: &mut editor) {
    let doc_id = ed.active_doc_id;
    let target_line = {
        let Some(doc) = ed.documents.get(&doc_id) else {
            return;
        };

        if doc.is_diff_too_large() {
            ed.status_message = Some("diff too large".to_string());
            return;
        }

        let hunks = doc.diff_hunks();
        if hunks.is_empty() {
            ed.status_message = Some("no diff hunks".to_string());
            return;
        }

        let cursor_char = doc.selection().primary().cursor(doc.text());
        let cursor_byte = doc.text().char_to_byte(cursor_char);
        let current_line = u32::try_from(doc.text().byte_to_line(cursor_byte))
            .expect("line number fits in u32");

        let target_hunk = hunks
            .iter()
            .rfind(|h| h.after_start < current_line)
            .or_else(|| hunks.last());

        match target_hunk {
            Some(h) => {
                let max_line = doc.text().line_count().saturating_sub(1);
                (h.after_start as usize).min(max_line)
            }
            None => return,
        }
    };

    let doc = ed.active_doc_mut();
    let target_byte = doc.text().line_to_byte(target_line).unwrap_or(0);
    let target_char = doc.text().byte_to_char(target_byte);
    doc.set_selection(selection::point(target_char));
    ed.ensure_active_cursor_visible();
}
