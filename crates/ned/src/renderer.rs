//! Frame renderer painting gutter, syntax tokens, selections, statusline, prompt, and popups.

use ned_config::cursor_shape;
use ned_config::line_number_style;
use ned_core::editor;
use ned_git::diff_tag;
use ned_lsp::diagnostic_severity;
use ned_tui::geometry::{point, Point};
use ned_tui::output::style::Style;
use ned_tui::Screen;

use crate::picker::picker_state;
use crate::prompt::prompt_state;
use crate::theme_bridge::{merge_style, style_for_scope};

/// Screen coordinates and shape for cursor positioning after frame paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct cursor_render_info {
    /// Screen column coordinate.
    pub col: u16,
    /// Screen row coordinate.
    pub row: u16,
    /// Desired terminal cursor shape.
    pub shape: cursor_shape,
    /// Whether the terminal cursor should be visible.
    pub visible: bool,
}

/// Computes the dynamic width of the gutter for `line_count`.
#[must_use]
pub fn calculate_gutter_width(line_count: usize) -> u16 {
    let digits = line_count.max(1).to_string().len().max(2);
    // Git diff symbol (1) + LSP diagnostic symbol (1) + Line digits + Space separator (1)
    (1 + 1 + digits + 1) as u16
}

/// Paints the entire editor frame to the double-buffered `Screen`.
///
/// Returns cursor positioning information for post-paint terminal synchronization.
///
/// # Panics
///
/// Panics if the active document or active view does not exist in the editor.
pub fn render_frame(
    screen: &mut Screen,
    ed: &mut editor,
    prompt: Option<&prompt_state>,
    picker: Option<&picker_state>,
) -> cursor_render_info {
    let Point {
        col: width,
        row: height,
    } = screen.size();
    if width < 5 || height < 2 {
        return cursor_render_info {
            col: 0,
            row: 0,
            shape: cursor_shape::block,
            visible: false,
        };
    }

    let editor_height = height.saturating_sub(2);
    let statusline_row = height.saturating_sub(2);
    let cmdline_row = height.saturating_sub(1);

    let doc_id = ed.active_doc_id;
    let view_id = ed.active_view_id;

    // Disjoint field access on `ed`
    let doc = ed
        .documents
        .get_mut(&doc_id)
        .expect("active document must exist");
    let total_lines = doc.text().line_count().max(1);
    let gutter_width = calculate_gutter_width(total_lines);
    let text_area_width = width.saturating_sub(gutter_width);

    let view = ed
        .views
        .get_mut(&view_id)
        .expect("active view must exist");
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
        let line_bg = if is_active_line && cursorline_enabled {
            merge_style(default_style, cursorline_style)
        } else {
            default_style
        };

        if line_idx < total_lines {
            // --- Gutter rendering ---
            // Column 0: Git diff symbol
            let (diff_sym, diff_style) = get_diff_indicator(
                doc.diff_hunks(),
                line_idx,
                gutter_style,
                diff_plus_style,
                diff_delta_style,
                diff_minus_style,
            );
            screen.set(point(0, row), &diff_style, diff_sym);

            // Column 1: LSP diagnostic indicator
            let (diag_sym, diag_style) = get_diagnostic_indicator(
                doc.diagnostics(),
                line_idx,
                gutter_style,
                diag_error_style,
                diag_warning_style,
                diag_info_style,
                diag_hint_style,
            );
            screen.set(point(1, row), &diag_style, diag_sym);

            // Columns 2..gutter_width-1: Line number
            let digits_width = (gutter_width as usize).saturating_sub(3);
            let num_str =
                format_line_number(line_idx, active_line_idx, line_style_cfg, digits_width);
            let num_style = if is_active_line {
                gutter_selected_style
            } else {
                gutter_style
            };

            for (i, ch) in num_str.chars().enumerate() {
                let col = (2 + i) as u16;
                let s = ch.to_string();
                screen.set(point(col, row), &num_style, &s);
            }
            // Gutter trailing spacer
            screen.set(point(gutter_width - 1, row), &num_style, " ");

            // --- Text Area rendering ---
            // Fill line background
            for c in gutter_width..width {
                screen.set(point(c, row), &line_bg, " ");
            }

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
                let mut token_style = line_bg;
                if let Some(span) = line_spans
                    .iter()
                    .find(|s| char_idx_in_line >= s.start && char_idx_in_line < s.end)
                {
                    let scope_style = style_for_scope(theme, span.scope());
                    token_style = merge_style(token_style, scope_style);
                }

                // Selection overlay
                for sel_range in doc.selection() {
                    let is_primary = *sel_range == primary_range;
                    if sel_range.contains(global_char_idx) {
                        let sel_overlay = if is_primary {
                            sel_primary_style
                        } else {
                            sel_secondary_style
                        };
                        token_style = merge_style(token_style, sel_overlay);
                        break;
                    }
                }

                let s = ch.to_string();
                screen.set(point(screen_col, row), &token_style, &s);
            }
        } else {
            // Beyond EOF: Helix virtual line indicator `~`
            screen.set(point(0, row), &gutter_style, " ");
            screen.set(point(1, row), &gutter_style, "~");
            for c in 2..width {
                screen.set(point(c, row), &default_style, " ");
            }
        }
    }

    // 3. Statusline (row `statusline_row`)
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

    // Right elements
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

    // 4. Command Line / Status Message (row `cmdline_row`)
    let cmdline_style = default_style;
    for c in 0..width {
        screen.set(point(c, cmdline_row), &cmdline_style, " ");
    }

    // 5. Command completion popup helper (when prompt is active and has candidate completions)
    if let Some(p) = prompt {
        let prompt_display = format!("{}{}", p.prefix, p.input);
        draw_str(screen, 0, cmdline_row, &prompt_display, &cmdline_style, width);

        if p.prefix == ":" {
            let completions = p.completions();
            if !completions.is_empty() {
                let menu_items_count = completions.len().min(8);
                let menu_height = menu_items_count as u16;
                let menu_width = 54.min(width.saturating_sub(4)).max(25);
                let menu_start_row = cmdline_row.saturating_sub(menu_height);
                let menu_start_col = 1u16;

                let menu_style = merge_style(default_style, style_for_scope(theme, "ui.menu"));
                let menu_sel_style =
                    merge_style(default_style, style_for_scope(theme, "ui.menu.selected"));

                for (i, item) in completions.iter().take(menu_items_count).enumerate() {
                    let r = menu_start_row + (i as u16);
                    let is_sel = p.completion_idx == Some(i);
                    let item_style = if is_sel {
                        &menu_sel_style
                    } else {
                        &menu_style
                    };

                    for c in 0..menu_width {
                        screen.set(point(menu_start_col + c, r), item_style, " ");
                    }

                    let name_display = format!(" {:<7} ", item.name);
                    let doc_col = draw_str(
                        screen,
                        menu_start_col,
                        r,
                        &name_display,
                        item_style,
                        menu_start_col + menu_width,
                    );
                    draw_str(
                        screen,
                        doc_col,
                        r,
                        item.doc,
                        item_style,
                        menu_start_col + menu_width,
                    );
                }
            }
        }

        let cursor_col = (p.prefix.chars().count() + p.cursor).min(width as usize) as u16;
        cursor_render_info {
            col: cursor_col,
            row: cmdline_row,
            shape: cursor_shape::bar,
            visible: true,
        }
    } else if let Some(p) = picker {
        // 6. Modal Picker Overlay Window (File picker / Buffer picker)
        let p_width = (width * 7 / 10).max(40).min(width.saturating_sub(4));
        let p_height = (height * 6 / 10).max(10).min(height.saturating_sub(2));
        let p_col = (width.saturating_sub(p_width)) / 2;
        let p_row = (height.saturating_sub(p_height)) / 2;

        let popup_style = merge_style(default_style, style_for_scope(theme, "ui.menu"));
        let popup_sel_style =
            merge_style(default_style, style_for_scope(theme, "ui.menu.selected"));
        let border_style = merge_style(popup_style, style_for_scope(theme, "ui.window"));

        // Clear popup area
        for r in 0..p_height {
            for c in 0..p_width {
                screen.set(point(p_col + c, p_row + r), &popup_style, " ");
            }
        }

        // Header line: title & match count
        let header_text = format!(" {} ({} matches) ", p.title, p.filtered.len());
        draw_str(
            screen,
            p_col + 1,
            p_row,
            &header_text,
            &border_style,
            p_col + p_width - 1,
        );

        // Query line
        let query_display = format!("> {}", p.query);
        draw_str(
            screen,
            p_col + 1,
            p_row + 1,
            &query_display,
            &popup_style,
            p_col + p_width - 1,
        );

        // Divider
        let divider = "─".repeat(p_width.saturating_sub(2) as usize);
        draw_str(
            screen,
            p_col + 1,
            p_row + 2,
            &divider,
            &border_style,
            p_col + p_width - 1,
        );

        // Items list
        let list_rows = (p_height.saturating_sub(3)) as usize;
        let scroll_offset = if p.selected >= list_rows {
            p.selected - list_rows + 1
        } else {
            0
        };

        for i in 0..list_rows {
            let item_filter_idx = scroll_offset + i;
            if item_filter_idx >= p.filtered.len() {
                break;
            }
            let item_idx = p.filtered[item_filter_idx];
            let item_text = &p.items[item_idx];
            let is_selected = item_filter_idx == p.selected;
            let item_style = if is_selected {
                &popup_sel_style
            } else {
                &popup_style
            };

            let row_y = p_row + 3 + (i as u16);
            for c in 1..p_width - 1 {
                screen.set(point(p_col + c, row_y), item_style, " ");
            }

            let display = format!(" {item_text}");
            draw_str(
                screen,
                p_col + 1,
                row_y,
                &display,
                item_style,
                p_col + p_width - 1,
            );
        }

        cursor_render_info {
            col: p_col + 3 + (p.query.chars().count() as u16),
            row: p_row + 1,
            shape: cursor_shape::bar,
            visible: true,
        }
    } else {
        if let Some(msg) = &ed.status_message {
            draw_str(screen, 0, cmdline_row, msg, &cmdline_style, width);
        }

        let (cursor_col, cursor_row, visible) =
            if let Some((c_col, c_row)) = view.cursor_coords(doc) {
                (gutter_width + c_col, c_row, true)
            } else {
                (gutter_width, 0, false)
            };

        let shape = match ed.mode {
            ned_config::mode::normal => ed.config.editor.cursor_shape.normal,
            ned_config::mode::insert => ed.config.editor.cursor_shape.insert,
            ned_config::mode::select => ed.config.editor.cursor_shape.select,
        };

        cursor_render_info {
            col: cursor_col,
            row: cursor_row,
            shape,
            visible,
        }
    }
}

/// Helper drawing a string horizontally onto `Screen` up to `max_col`.
fn draw_str(
    screen: &mut Screen,
    mut col: u16,
    row: u16,
    text: &str,
    style: &Style,
    max_col: u16,
) -> u16 {
    for ch in text.chars() {
        if col >= max_col {
            break;
        }
        let s = ch.to_string();
        screen.set(point(col, row), style, &s);
        col += 1;
    }
    col
}

/// Computes the diff indicator and style for a given line index.
fn get_diff_indicator(
    hunks: &[ned_git::hunk],
    line_idx: usize,
    gutter_style: Style,
    plus_style: Style,
    delta_style: Style,
    minus_style: Style,
) -> (&'static str, Style) {
    for h in hunks {
        let start = h.after_start as usize;
        let count = h.after_count.max(1) as usize;
        if line_idx >= start && line_idx < start + count {
            return match h.kind {
                diff_tag::added => ("+", plus_style),
                diff_tag::modified => ("~", delta_style),
                diff_tag::deleted => ("-", minus_style),
            };
        }
    }
    (" ", gutter_style)
}

/// Computes the LSP diagnostic indicator and style for a given line index.
fn get_diagnostic_indicator(
    diags: &[ned_lsp::diagnostic],
    line_idx: usize,
    gutter_style: Style,
    error_style: Style,
    warning_style: Style,
    info_style: Style,
    hint_style: Style,
) -> (&'static str, Style) {
    let mut highest_severity: Option<diagnostic_severity> = None;

    for d in diags {
        if d.range.start.line as usize == line_idx {
            match d.severity {
                Some(diagnostic_severity::error) => {
                    return ("●", error_style);
                }
                Some(diagnostic_severity::warning) => {
                    highest_severity = Some(diagnostic_severity::warning);
                }
                Some(diagnostic_severity::information) => {
                    if highest_severity.is_none()
                        || highest_severity == Some(diagnostic_severity::hint)
                    {
                        highest_severity = Some(diagnostic_severity::information);
                    }
                }
                Some(diagnostic_severity::hint) => {
                    if highest_severity.is_none() {
                        highest_severity = Some(diagnostic_severity::hint);
                    }
                }
                None => {}
            }
        }
    }

    match highest_severity {
        Some(diagnostic_severity::error) => ("●", error_style),
        Some(diagnostic_severity::warning) => ("●", warning_style),
        Some(diagnostic_severity::information) => ("●", info_style),
        Some(diagnostic_severity::hint) => ("●", hint_style),
        None => (" ", gutter_style),
    }
}

/// Formats the line number string respecting relative and absolute settings.
fn format_line_number(
    line_idx: usize,
    active_idx: usize,
    style: line_number_style,
    width: usize,
) -> String {
    match style {
        line_number_style::none => " ".repeat(width),
        line_number_style::absolute => {
            format!("{:>width$}", line_idx + 1)
        }
        line_number_style::relative => {
            if line_idx == active_idx {
                format!("{:>width$}", line_idx + 1)
            } else {
                let diff = (line_idx as isize - active_idx as isize).unsigned_abs();
                format!("{diff:>width$}")
            }
        }
    }
}
