//! Editor view rendering painting gutter, line numbers, diffs, diagnostics, and syntax tokens.

use ned_core::editor;
use ned_tui::geometry::point;
use ned_tui::Screen;

use crate::theme_bridge::{merge_style, style_for_scope};
use crate::ui::draw_str;

pub mod buffer;
pub mod cursor;
pub mod diagnostics;
pub mod gutter;
pub mod selection;

pub use gutter::calculate_gutter_width;

/// Renders the editor lines, gutter, syntax tokens, and selection overlays.
///
/// # Panics
///
/// Panics if the active document or view does not exist in the editor.
pub fn render_editor(
    screen: &mut Screen,
    ed: &mut editor,
    width: u16,
    editor_height: u16,
    gutter_width: u16,
) {
    let doc_id = ed.active_doc_id;
    let view_id = ed.active_view_id;

    let doc = ed
        .documents
        .get_mut(&doc_id)
        .expect("active document must exist");
    let total_lines = doc.text().line_count().max(1);
    let text_area_width = width.saturating_sub(gutter_width);

    let view = ed.views.get_mut(&view_id).expect("active view must exist");
    view.resize(text_area_width, editor_height);
    view.ensure_cursor_visible(doc, ed.config.editor.scrolloff);

    // 1. Resolve theme styles
    let theme = &ed.config.theme;
    let base_text_style = style_for_scope(theme, "ui.text");
    let base_bg_style = style_for_scope(theme, "ui.background");
    let default_style = merge_style(base_bg_style, base_text_style);

    let gutter_style = merge_style(default_style, style_for_scope(theme, "ui.gutter"));
    let gutter_selected_style =
        merge_style(gutter_style, style_for_scope(theme, "ui.gutter.selected"));
    let cursorline_style = style_for_scope(theme, "ui.cursorline.primary");

    let sel_primary_style = style_for_scope(theme, "ui.selection.primary");
    let sel_secondary_style = style_for_scope(theme, "ui.selection");

    let diff_plus_style = merge_style(gutter_style, style_for_scope(theme, "diff.plus"));
    let diff_delta_style = merge_style(gutter_style, style_for_scope(theme, "diff.delta"));
    let diff_minus_style = merge_style(gutter_style, style_for_scope(theme, "diff.minus"));

    let diag_error_style = merge_style(gutter_style, style_for_scope(theme, "diagnostic.error"));
    let diag_warning_style =
        merge_style(gutter_style, style_for_scope(theme, "diagnostic.warning"));
    let diag_info_style = merge_style(gutter_style, style_for_scope(theme, "diagnostic.info"));
    let diag_hint_style = merge_style(gutter_style, style_for_scope(theme, "diagnostic.hint"));

    let primary_cursor_char = doc.selection().primary().cursor(doc.text());
    let primary_cursor_byte = doc.text().char_to_byte(primary_cursor_char);
    let active_line_idx = doc.text().byte_to_line(primary_cursor_byte);

    let start_line = view.row_offset;
    let end_line = start_line + (editor_height as usize);
    let highlighted_lines = doc.highlight_viewport(start_line, end_line);

    let col_offset = view.col_offset;
    let line_style_cfg = ed.config.editor.line_number;
    let cursorline_enabled = ed.config.editor.cursor_line;

    // 2. Render editor lines (Gutter + Text area)
    for row in 0..editor_height {
        let line_idx = start_line + (row as usize);
        let is_active_line = line_idx == active_line_idx;
        let line_bg = cursor::resolve_line_background(
            is_active_line,
            cursorline_enabled,
            default_style,
            cursorline_style,
        );

        if line_idx < total_lines {
            // --- Gutter rendering (Helix layout: [diag] [line-numbers] [space] [diff]) ---
            let (diag_sym, diag_style) = diagnostics::get_diagnostic_indicator(
                doc.diagnostics(),
                line_idx,
                gutter_style,
                diag_error_style,
                diag_warning_style,
                diag_info_style,
                diag_hint_style,
            );
            let digits_width = (gutter_width as usize).saturating_sub(3);
            let num_str = gutter::format_line_number(
                line_idx,
                active_line_idx,
                line_style_cfg,
                digits_width,
                ed.mode,
            );
            let num_style = if is_active_line {
                gutter_selected_style
            } else {
                gutter_style
            };
            let (diff_sym, diff_style) = gutter::get_diff_indicator(
                doc.diff_hunks(),
                line_idx,
                total_lines,
                gutter_style,
                diff_plus_style,
                diff_delta_style,
                diff_minus_style,
            );
            gutter::render_gutter_row(
                screen,
                row,
                gutter_width,
                diag_sym,
                &diag_style,
                &num_str,
                &num_style,
                diff_sym,
                &diff_style,
            );

            // --- Text Area rendering ---
            // Fill line background
            buffer::fill_line_background(screen, row, gutter_width, width, &line_bg);

            let line_str = doc.text().line(line_idx).unwrap_or_default();
            let clean_str = line_str.trim_end_matches(['\r', '\n']);
            let line_chars: Vec<char> = clean_str.chars().collect();

            // Find syntax spans for this line
            let line_spans = highlighted_lines
                .iter()
                .find(|(l, _)| *l == line_idx)
                .map_or(&[][..], |(_, spans)| spans.as_slice());

            let line_start_byte = doc.text().line_to_byte(line_idx).unwrap_or(0);
            let line_start_char = doc.text().byte_to_char(line_start_byte);
            let primary_range = doc.selection().primary();

            for (char_idx_in_line, &ch) in line_chars.iter().enumerate() {
                if char_idx_in_line < col_offset {
                    continue;
                }
                let visual_col = (char_idx_in_line - col_offset) as u16;
                if visual_col >= text_area_width {
                    break;
                }

                let screen_col = gutter_width + visual_col;
                let global_char_idx = line_start_char + char_idx_in_line;

                // Base token style from syntax highlighter
                let mut token_style =
                    buffer::resolve_syntax_style(theme, line_spans, char_idx_in_line, line_bg);

                // Inline diagnostic underline
                if let Some(ul_color) = diagnostics::diag_underline_color(
                    doc.diagnostics(),
                    line_idx,
                    char_idx_in_line,
                    diag_error_style,
                    diag_warning_style,
                    diag_info_style,
                    diag_hint_style,
                ) {
                    token_style.underlined = Some(true);
                    token_style.frame_color = Some(ul_color);
                }

                // Selection overlay
                token_style = selection::apply_selection_style(
                    doc.selection(),
                    primary_range,
                    global_char_idx,
                    token_style,
                    sel_primary_style,
                    sel_secondary_style,
                );

                let s = ch.to_string();
                screen.set(point(screen_col, row), &token_style, &s);
            }

            // Inline end-of-line diagnostic message (Helix inline-diagnostics parity)
            if let Some((diag_msg, diag_fg_style)) = diagnostics::get_line_inline_diagnostic(
                doc.diagnostics(),
                line_idx,
                diag_error_style,
                diag_warning_style,
                diag_info_style,
                diag_hint_style,
            ) {
                let visible_len = line_chars.len().saturating_sub(col_offset) as u16;
                let eol_col = gutter_width + visible_len + 2;
                if eol_col + 4 < width {
                    let mut inline_style = merge_style(line_bg, diag_fg_style);
                    inline_style.italic = Some(true);
                    let first_line = diag_msg.lines().next().unwrap_or("");
                    let display_msg = format!("■ {first_line}");
                    draw_str(screen, eol_col, row, &display_msg, &inline_style, width);
                }
            }
        } else {
            // Beyond EOF: Helix virtual line indicator `~`
            buffer::render_beyond_eof(
                screen,
                row,
                gutter_width,
                width,
                &gutter_style,
                &default_style,
            );
        }
    }
}
