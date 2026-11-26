//! Statusline rendering displaying mode, file status, LSP diagnostics, and cursor coordinates.

use ned_core::editor;
use ned_lsp::diagnostic_severity;
use ned_tui::geometry::point;
use ned_tui::Screen;

use crate::theme_bridge::{merge_style, style_for_scope};
use crate::ui::draw_str;

/// Paints the statusline onto the screen.
///
/// # Panics
///
/// Panics if the active document does not exist in the editor.
pub fn render_statusline(screen: &mut Screen, ed: &editor, width: u16, statusline_row: u16) {
    let theme = &ed.config.theme;
    let base_text_style = style_for_scope(theme, "ui.text");
    let base_bg_style = style_for_scope(theme, "ui.background");
    let default_style = merge_style(base_bg_style, base_text_style);
    let statusline_style = merge_style(default_style, style_for_scope(theme, "ui.statusline"));

    for c in 0..width {
        screen.set(point(c, statusline_row), &statusline_style, " ");
    }

    let mode_str = match ed.mode {
        ned_config::mode::normal => " NOR ",
        ned_config::mode::insert => " INS ",
        ned_config::mode::select => " SEL ",
    };
    let mode_scope = match ed.mode {
        ned_config::mode::normal => "ui.statusline.normal",
        ned_config::mode::insert => "ui.statusline.insert",
        ned_config::mode::select => "ui.statusline.select",
    };
    let mode_style = merge_style(statusline_style, style_for_scope(theme, mode_scope));

    let mut left_col = 0u16;
    left_col = draw_str(
        screen,
        left_col,
        statusline_row,
        mode_str,
        &mode_style,
        width,
    );

    let doc = ed
        .documents
        .get(&ed.active_doc_id)
        .expect("active document must exist");

    let file_title = doc
        .path()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("[scratch]");

    let file_display = if doc.is_modified() {
        format!(" {file_title} [+] ")
    } else {
        format!(" {file_title} ")
    };
    left_col = draw_str(
        screen,
        left_col,
        statusline_row,
        &file_display,
        &statusline_style,
        width,
    );

    // Diagnostic summary in statusline
    let mut err_count = 0usize;
    let mut warn_count = 0usize;
    for d in doc.diagnostics() {
        match d.severity {
            Some(diagnostic_severity::error) => err_count += 1,
            Some(diagnostic_severity::warning) => warn_count += 1,
            _ => {}
        }
    }
    if err_count > 0 || warn_count > 0 {
        let diag_error_style = style_for_scope(theme, "diagnostic.error");
        let diag_warning_style = style_for_scope(theme, "diagnostic.warning");
        let diag_str = format!(" ● {err_count}E {warn_count}W ");
        let diag_style = if err_count > 0 {
            merge_style(statusline_style, diag_error_style)
        } else {
            merge_style(statusline_style, diag_warning_style)
        };
        draw_str(
            screen,
            left_col,
            statusline_row,
            &diag_str,
            &diag_style,
            width,
        );
    }

    // Right elements (line:col, encoding, line ending, multi-selection)
    let primary_cursor_char = doc.selection().primary().cursor(doc.text());
    let primary_cursor_byte = doc.text().char_to_byte(primary_cursor_char);
    let active_line_idx = doc.text().byte_to_line(primary_cursor_byte);

    let line_start_byte = doc.text().line_to_byte(active_line_idx).unwrap_or(0);
    let line_start_char = doc.text().byte_to_char(line_start_byte);
    let active_col_idx = primary_cursor_char.saturating_sub(line_start_char);

    let sel_info = if doc.selection().len() > 1 {
        format!("{} sel ", doc.selection().len())
    } else {
        String::new()
    };
    let line_end_str = match doc.line_ending() {
        ned_text::line_ending::crlf => "CRLF",
        ned_text::line_ending::lf => "LF",
    };
    let right_text = format!(
        "{sel_info}{}:{}  utf-8  {line_end_str} ",
        active_line_idx + 1,
        active_col_idx + 1
    );
    let right_width = right_text.chars().count() as u16;
    let right_col = width.saturating_sub(right_width);
    draw_str(
        screen,
        right_col,
        statusline_row,
        &right_text,
        &statusline_style,
        width,
    );
}
