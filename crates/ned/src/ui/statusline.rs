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
        .and_then(|p| {
            if let Ok(cwd) = std::env::current_dir() {
                if let Ok(rel) = p.strip_prefix(&cwd) {
                    return Some(rel.to_string_lossy().into_owned());
                }
            }
            p.file_name()
                .and_then(|s| s.to_str())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "[scratch]".to_string());

    let mod_indicator = if doc.is_modified() { "[+]" } else { "   " };
    let file_display = format!(" {file_title} {mod_indicator} ");
    draw_str(
        screen,
        left_col,
        statusline_row,
        &file_display,
        &statusline_style,
        width,
    );

    // Right elements matching Helix layout: [Diagnostics] [Selections] [Position]
    let mut right_col = width;

    // 1. Position: " line:col "
    let primary_cursor_char = doc.selection().primary().cursor(doc.text());
    let primary_cursor_byte = doc.text().char_to_byte(primary_cursor_char);
    let active_line_idx = doc.text().byte_to_line(primary_cursor_byte);

    let line_start_byte = doc.text().line_to_byte(active_line_idx).unwrap_or(0);
    let line_start_char = doc.text().byte_to_char(line_start_byte);
    let active_col_idx = primary_cursor_char.saturating_sub(line_start_char);

    let pos_text = format!(" {}:{} ", active_line_idx + 1, active_col_idx + 1);
    let pos_width = pos_text.chars().count() as u16;
    right_col = right_col.saturating_sub(pos_width);
    draw_str(
        screen,
        right_col,
        statusline_row,
        &pos_text,
        &statusline_style,
        width,
    );

    // 2. Selections: " 1 sel " or " 1/2 sels "
    let sel_count = doc.selection().len();
    let sel_text = if sel_count == 1 {
        " 1 sel ".to_string()
    } else {
        let primary_idx = doc.selection().primary_index() + 1;
        format!(" {primary_idx}/{sel_count} sels ")
    };
    let sel_width = sel_text.chars().count() as u16;
    right_col = right_col.saturating_sub(sel_width);
    draw_str(
        screen,
        right_col,
        statusline_row,
        &sel_text,
        &statusline_style,
        width,
    );

    // 3. Diagnostics: "● {warnings} ● {errors}"
    let mut err_count = 0usize;
    let mut warn_count = 0usize;
    for d in doc.diagnostics() {
        match d.severity {
            Some(diagnostic_severity::error) => err_count += 1,
            Some(diagnostic_severity::warning) => warn_count += 1,
            _ => {}
        }
    }

    if err_count > 0 {
        let err_text = format!(" {err_count} ");
        let err_width = err_text.chars().count() as u16 + 1;
        right_col = right_col.saturating_sub(err_width);
        let diag_err_style =
            merge_style(statusline_style, style_for_scope(theme, "diagnostic.error"));
        let col = draw_str(
            screen,
            right_col,
            statusline_row,
            "●",
            &diag_err_style,
            width,
        );
        draw_str(
            screen,
            col,
            statusline_row,
            &err_text,
            &statusline_style,
            width,
        );
    }

    if warn_count > 0 {
        let warn_text = format!(" {warn_count} ");
        let warn_width = warn_text.chars().count() as u16 + 1;
        right_col = right_col.saturating_sub(warn_width);
        let diag_warn_style = merge_style(
            statusline_style,
            style_for_scope(theme, "diagnostic.warning"),
        );
        let col = draw_str(
            screen,
            right_col,
            statusline_row,
            "●",
            &diag_warn_style,
            width,
        );
        draw_str(
            screen,
            col,
            statusline_row,
            &warn_text,
            &statusline_style,
            width,
        );
    }
}
