//! Editor view rendering painting gutter, line numbers, diffs, diagnostics, and syntax tokens.

use ned_config::line_number_style;
use ned_core::editor;
use ned_git::diff_tag;
use ned_lsp::diagnostic_severity;
use ned_tui::geometry::point;
use ned_tui::output::color::Color;
use ned_tui::output::style::Style;
use ned_tui::Screen;

use crate::theme_bridge::{merge_style, style_for_scope};
use crate::ui::draw_str;

/// Computes the dynamic width of the gutter for `line_count` matching Helix layout:
/// `[diagnostic (1)] [line_digits] [space (1)] [diff indicator (1)]`.
#[must_use]
pub fn calculate_gutter_width(line_count: usize) -> u16 {
    let digits = line_count.max(1).to_string().len().max(2);
    // LSP diagnostic symbol (1) + Line digits + Space separator (1) + Git diff indicator (1)
    (1 + digits + 1 + 1) as u16
}

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
            // --- Gutter rendering (Helix layout: [diag] [line-numbers] [space] [diff]) ---
            // Column 0: LSP diagnostic indicator
            let (diag_sym, diag_style) = get_diagnostic_indicator(
                doc.diagnostics(),
                line_idx,
                gutter_style,
                diag_error_style,
                diag_warning_style,
                diag_info_style,
                diag_hint_style,
            );
            screen.set(point(0, row), &diag_style, diag_sym);

            // Columns 1..gutter_width-2: Line number
            let digits_width = (gutter_width as usize).saturating_sub(3);
            let num_str =
                format_line_number(line_idx, active_line_idx, line_style_cfg, digits_width);
            let num_style = if is_active_line {
                gutter_selected_style
            } else {
                gutter_style
            };

            for (i, ch) in num_str.chars().enumerate() {
                let col = (1 + i) as u16;
                let s = ch.to_string();
                screen.set(point(col, row), &num_style, &s);
            }

            // Column gutter_width - 2: Space separator on left side of diff bar
            screen.set(point(gutter_width - 2, row), &num_style, " ");

            // Column gutter_width - 1: Git diff indicator (adjacent to code area)
            let (diff_sym, diff_style) = get_diff_indicator(
                doc.diff_hunks(),
                line_idx,
                total_lines,
                gutter_style,
                diff_plus_style,
                diff_delta_style,
                diff_minus_style,
            );
            screen.set(point(gutter_width - 1, row), &diff_style, diff_sym);

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

                // Inline diagnostic underline
                if let Some(ul_color) = diag_underline_color(
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

            // Inline end-of-line diagnostic message (Helix inline-diagnostics parity)
            if let Some((diag_msg, diag_fg_style)) = get_line_inline_diagnostic(
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
            screen.set(point(0, row), &gutter_style, " ");
            screen.set(point(1, row), &gutter_style, "~");
            for c in 2..gutter_width {
                screen.set(point(c, row), &gutter_style, " ");
            }
            for c in gutter_width..width {
                screen.set(point(c, row), &default_style, " ");
            }
        }
    }
}

/// Computes the diff indicator and style for a given line index matching Helix parity.
///
/// Symbols:
/// - Added: `▎` (left 1/4 block) with `diff.plus` (green)
/// - Modified: `▍` (left 3/8 block) with `diff.delta` (yellow)
/// - Deleted: `▔` (upper 1/8 block) with `diff.minus` (red)
fn get_diff_indicator(
    hunks: &[ned_git::hunk],
    line_idx: usize,
    total_lines: usize,
    gutter_style: Style,
    plus_style: Style,
    delta_style: Style,
    minus_style: Style,
) -> (&'static str, Style) {
    for h in hunks {
        let start = h.after_start as usize;
        let count = h.after_count as usize;

        match h.kind {
            diff_tag::added => {
                if count > 0 && line_idx >= start && line_idx < start + count {
                    return ("▎", plus_style);
                }
            }
            diff_tag::modified => {
                if count > 0 && line_idx >= start && line_idx < start + count {
                    return ("▍", delta_style);
                }
            }
            diff_tag::deleted => {
                if line_idx == start || (start >= total_lines && line_idx + 1 == total_lines) {
                    return ("▔", minus_style);
                }
            }
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

/// Returns the underline color for a character inside a diagnostic range, prioritizing errors.
fn diag_underline_color(
    diags: &[ned_lsp::diagnostic],
    line_idx: usize,
    col_idx: usize,
    error_style: Style,
    warning_style: Style,
    info_style: Style,
    hint_style: Style,
) -> Option<Color> {
    let l = line_idx as u32;
    let c = col_idx as u32;
    let mut best: Option<(u8, Style)> = None;

    for d in diags {
        let s = &d.range.start;
        let e = &d.range.end;
        let in_range = if s.line == e.line {
            l == s.line
                && c >= s.character
                && (c < e.character || (s.character == e.character && c == s.character))
        } else {
            (l == s.line && c >= s.character)
                || (l > s.line && l < e.line)
                || (l == e.line && c < e.character)
        };

        if in_range {
            let (prio, st) = match d.severity {
                Some(diagnostic_severity::error) => (4, error_style),
                Some(diagnostic_severity::warning) => (3, warning_style),
                Some(diagnostic_severity::information) => (2, info_style),
                Some(diagnostic_severity::hint) | None => (1, hint_style),
            };
            if best.is_none_or(|(bp, _)| prio > bp) {
                best = Some((prio, st));
            }
        }
    }

    best.and_then(|(_, st)| st.front_color)
}

/// Returns the highest-severity diagnostic message and style for inline end-of-line display.
fn get_line_inline_diagnostic(
    diags: &[ned_lsp::diagnostic],
    line_idx: usize,
    error_style: Style,
    warning_style: Style,
    info_style: Style,
    hint_style: Style,
) -> Option<(&str, Style)> {
    let mut best: Option<(u8, &str, Style)> = None;

    for d in diags {
        if d.range.start.line as usize == line_idx {
            let (prio, st) = match d.severity {
                Some(diagnostic_severity::error) => (4, error_style),
                Some(diagnostic_severity::warning) => (3, warning_style),
                Some(diagnostic_severity::information) => (2, info_style),
                Some(diagnostic_severity::hint) | None => (1, hint_style),
            };
            if best.is_none_or(|(bp, _, _)| prio > bp) {
                best = Some((prio, d.message.as_str(), st));
            }
        }
    }

    best.map(|(_, msg, st)| (msg, st))
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
