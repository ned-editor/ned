//! Frame renderer painting gutter, syntax tokens, selections, statusline, prompt, and popups.

use ned_config::cursor_shape;
use ned_config::line_number_style;
use ned_core::editor;
use ned_git::diff_tag;
use ned_lsp::diagnostic_severity;
use ned_tui::geometry::{point, Point};
use ned_tui::output::style::Style;
use ned_tui::Screen;

use crate::picker::{min_area_width_for_preview, picker_state, preview_content};
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
            let completions = &p.completion;
            let base_width = 30u16;

            let completion_height = if completions.is_empty() {
                0
            } else {
                let max_len = completions
                    .iter()
                    .map(|c| c.name.len() as u16)
                    .max()
                    .unwrap_or(base_width)
                    .max(base_width);

                let cols = (width / max_len).max(1);
                let col_width = (width.saturating_sub(cols)) / cols;
                let height = (completions.len() as u16)
                    .div_ceil(cols)
                    .min(10)
                    .min(cmdline_row);

                let completion_start_row = cmdline_row.saturating_sub(height);
                let menu_style = merge_style(default_style, style_for_scope(theme, "ui.menu"));
                let menu_sel_style =
                    merge_style(default_style, style_for_scope(theme, "ui.menu.selected"));

                let items = (height as usize) * (cols as usize);
                let offset = p
                    .selection
                    .map(|idx| (idx / items) * items)
                    .unwrap_or(0);

                // Clear completion grid area
                for r in 0..height {
                    for c in 0..width {
                        screen.set(point(c, completion_start_row + r), &menu_style, " ");
                    }
                }

                // Render grid items column by column
                let mut row = 0u16;
                let mut col = 0u16;

                for (i, item) in completions.iter().enumerate().skip(offset).take(items) {
                    let is_sel = p.selection == Some(i);
                    let item_style = if is_sel {
                        &menu_sel_style
                    } else {
                        &menu_style
                    };

                    let x = col * (1 + col_width);
                    let y = completion_start_row + row;

                    if is_sel {
                        for c_offset in 0..col_width {
                            screen.set(point(x + c_offset, y), item_style, " ");
                        }
                    }

                    draw_str(screen, x, y, item.name, item_style, x + col_width);

                    row += 1;
                    if row >= height {
                        row = 0;
                        col += 1;
                    }
                }

                height
            };

            // Render documentation box directly above completion area
            if let Some((cmd_name, doc_text)) = p.active_doc() {
                let max_box_width = ((base_width * 3) as usize).min((width as usize).saturating_sub(2));
                let inner_text_width = max_box_width.saturating_sub(4).max(10);
                let full_doc = format!("{cmd_name}: {doc_text}");
                let wrapped_lines = wrap_text(&full_doc, inner_text_width);

                let doc_box_width = ((wrapped_lines.iter().map(String::len).max().unwrap_or(20) + 4) as u16)
                    .min(width)
                    .max(25);
                let doc_box_height = (wrapped_lines.len() as u16 + 2).min(cmdline_row.saturating_sub(completion_height));

                if doc_box_height >= 3 {
                    let doc_box_row = cmdline_row
                        .saturating_sub(completion_height)
                        .saturating_sub(doc_box_height);
                    let doc_box_col = 0u16;

                    let help_style = merge_style(default_style, style_for_scope(theme, "ui.help"));

                    // Clear doc box area
                    for r in 0..doc_box_height {
                        for c in 0..doc_box_width {
                            screen.set(point(doc_box_col + c, doc_box_row + r), &help_style, " ");
                        }
                    }

                    // Render border
                    let last_c = doc_box_width.saturating_sub(1);
                    let last_r = doc_box_height.saturating_sub(1);

                    screen.set(point(0, doc_box_row), &help_style, "┌");
                    screen.set(point(last_c, doc_box_row), &help_style, "┐");
                    screen.set(point(0, doc_box_row + last_r), &help_style, "└");
                    screen.set(point(last_c, doc_box_row + last_r), &help_style, "┘");

                    for c in 1..last_c {
                        screen.set(point(c, doc_box_row), &help_style, "─");
                        screen.set(point(c, doc_box_row + last_r), &help_style, "─");
                    }
                    for r in 1..last_r {
                        screen.set(point(0, doc_box_row + r), &help_style, "│");
                        screen.set(point(last_c, doc_box_row + r), &help_style, "│");
                    }

                    // Render wrapped documentation text
                    for (line_idx, line) in wrapped_lines
                        .iter()
                        .take((last_r.saturating_sub(1)) as usize)
                        .enumerate()
                    {
                        draw_str(
                            screen,
                            doc_box_col + 2,
                            doc_box_row + 1 + (line_idx as u16),
                            line,
                            &help_style,
                            doc_box_col + last_c,
                        );
                    }
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
        // 6. Modal Picker Overlay Window (File picker / Buffer picker) with live split preview
        let popup_style = merge_style(default_style, style_for_scope(theme, "ui.menu"));
        let popup_sel_style =
            merge_style(default_style, style_for_scope(theme, "ui.menu.selected"));
        let border_style = merge_style(popup_style, style_for_scope(theme, "ui.window"));
        let text_style = merge_style(default_style, style_for_scope(theme, "ui.text"));
        let linenr_style = merge_style(default_style, style_for_scope(theme, "ui.linenr"));
        let sep_style =
            merge_style(border_style, style_for_scope(theme, "ui.background.separator"));
        let inactive_style =
            merge_style(default_style, style_for_scope(theme, "ui.text.inactive"));
        let match_style = merge_style(
            style_for_scope(theme, "special"),
            style_for_scope(theme, "ui.text.focus"),
        );

        let fits_beside = width > min_area_width_for_preview && p.show_preview;
        let picker_width = if fits_beside { width / 2 } else { width };
        let preview_width = if fits_beside { width - picker_width } else { 0 };

        // --- Render Left Pane (Picker candidate list & search query) ---
        // Clear picker area
        for r in 0..height {
            for c in 0..picker_width {
                screen.set(point(c, r), &popup_style, " ");
            }
        }

        // Picker top border
        screen.set(point(0, 0), &border_style, "┌");
        screen.set(point(picker_width - 1, 0), &border_style, "┐");
        for c in 1..picker_width - 1 {
            screen.set(point(c, 0), &border_style, "─");
        }

        // Picker prompt line (row 1)
        screen.set(point(0, 1), &border_style, "│");
        screen.set(point(picker_width - 1, 1), &border_style, "│");
        let query_prompt = format!("> {}", p.query);
        draw_str(
            screen,
            2,
            1,
            &query_prompt,
            &text_style,
            picker_width.saturating_sub(12),
        );

        let count_str = format!("{}/{}", p.filtered.len(), p.items.len());
        let count_col = (picker_width - 1).saturating_sub(count_str.len() as u16 + 1);
        if count_col > 4 + p.query.chars().count() as u16 {
            draw_str(screen, count_col, 1, &count_str, &text_style, picker_width - 1);
        }

        // Picker separator line (row 2)
        screen.set(point(0, 2), &border_style, "├");
        screen.set(point(picker_width - 1, 2), &border_style, "┤");
        for c in 1..picker_width - 1 {
            screen.set(point(c, 2), &sep_style, "─");
        }

        // Picker items list (rows 3 .. height - 1)
        let list_height = height.saturating_sub(4) as usize;
        let scroll_offset = if p.selected >= list_height {
            p.selected - list_height + 1
        } else {
            0
        };

        for i in 0..list_height {
            let row_y = 3 + i as u16;
            screen.set(point(0, row_y), &border_style, "│");
            screen.set(point(picker_width - 1, row_y), &border_style, "│");

            let filter_idx = scroll_offset + i;
            if filter_idx < p.filtered.len() {
                let item_idx = p.filtered[filter_idx];
                let item_text = &p.items[item_idx];
                let is_selected = filter_idx == p.selected;
                let row_style = if is_selected {
                    &popup_sel_style
                } else {
                    &popup_style
                };

                for c in 1..picker_width - 1 {
                    screen.set(point(c, row_y), row_style, " ");
                }

                let prefix = if is_selected { " > " } else { "   " };
                draw_str(screen, 1, row_y, prefix, row_style, picker_width - 1);

                let text_start_x = 1 + prefix.len() as u16;
                let max_text_x = picker_width.saturating_sub(1);
                let match_indices = find_subsequence_matches(item_text, &p.query);

                for (char_idx, ch) in item_text.chars().enumerate() {
                    let char_x = text_start_x + char_idx as u16;
                    if char_x >= max_text_x {
                        break;
                    }
                    let is_match = match_indices.contains(&char_idx);
                    let char_style = if is_match {
                        merge_style(*row_style, match_style)
                    } else {
                        *row_style
                    };
                    screen.set(point(char_x, row_y), &char_style, &ch.to_string());
                }
            }
        }

        // Picker bottom border
        let bottom_y = height.saturating_sub(1);
        screen.set(point(0, bottom_y), &border_style, "└");
        screen.set(point(picker_width - 1, bottom_y), &border_style, "┘");
        for c in 1..picker_width - 1 {
            screen.set(point(c, bottom_y), &border_style, "─");
        }

        // --- Render Right Pane (Live file preview) ---
        if fits_beside && preview_width > 4 {
            let prev_start = picker_width;
            let prev_end = width.saturating_sub(1);

            // Clear preview area
            for r in 0..height {
                for c in prev_start..=prev_end {
                    screen.set(point(c, r), &popup_style, " ");
                }
            }

            // Preview top border
            screen.set(point(prev_start, 0), &border_style, "┌");
            screen.set(point(prev_end, 0), &border_style, "┐");
            for c in prev_start + 1..prev_end {
                screen.set(point(c, 0), &border_style, "─");
            }

            // Preview side borders
            for r in 1..bottom_y {
                screen.set(point(prev_start, r), &border_style, "│");
                screen.set(point(prev_end, r), &border_style, "│");
            }

            // Preview bottom border
            screen.set(point(prev_start, bottom_y), &border_style, "└");
            screen.set(point(prev_end, bottom_y), &border_style, "┘");
            for c in prev_start + 1..prev_end {
                screen.set(point(c, bottom_y), &border_style, "─");
            }

            // Preview content
            let inner_x = prev_start + 2;
            let inner_w = prev_end.saturating_sub(prev_start + 2);
            let inner_h = height.saturating_sub(2);

            if let Some(preview) = p.get_preview() {
                if let Some(placeholder) = preview.placeholder() {
                    let px = inner_x + (inner_w.saturating_sub(placeholder.len() as u16)) / 2;
                    let py = 1 + inner_h / 2;
                    draw_str(screen, px, py, placeholder, &inactive_style, prev_end);
                } else {
                    match preview {
                        preview_content::directory(entries) => {
                            for (i, (name, is_dir)) in
                                entries.iter().take(inner_h as usize).enumerate()
                            {
                                let y = 1 + i as u16;
                                let display =
                                    if *is_dir { format!("{name}/") } else { name.clone() };
                                let style = if *is_dir {
                                    &popup_sel_style
                                } else {
                                    &text_style
                                };
                                draw_str(screen, inner_x, y, &display, style, prev_end - 1);
                            }
                        }
                        preview_content::text(lines) => {
                            if lines.is_empty() {
                                let empty_msg = "<Empty file>";
                                let px =
                                    inner_x + (inner_w.saturating_sub(empty_msg.len() as u16)) / 2;
                                let py = 1 + inner_h / 2;
                                draw_str(screen, px, py, empty_msg, &inactive_style, prev_end);
                            } else {
                                let preview_path = p.selected_item().map(|item| {
                                    if let Some(root) = &p.root_dir {
                                        root.join(item)
                                    } else {
                                        std::path::PathBuf::from(item)
                                    }
                                });

                                let lang_cfg = preview_path
                                    .as_deref()
                                    .and_then(|path| ed.config.languages.detect_by_path(path));
                                let grammar = lang_cfg
                                    .and_then(|cfg| ned_core::load_grammar(&cfg.name));

                                let highlighted_preview = if let Some(grammar) = grammar {
                                    let mut highlighter =
                                        ned_tree::syntax_highlighter::new(Some(grammar));
                                    let preview_text = lines.join("\n");
                                    let rope_buf = ned_rope::rope::from_str(&preview_text);
                                    highlighter
                                        .highlight_viewport(&rope_buf, 0, lines.len().min(inner_h as usize))
                                } else {
                                    Vec::new()
                                };

                                for (i, line) in lines.iter().take(inner_h as usize).enumerate() {
                                    let y = 1 + i as u16;
                                    let line_nr_str = format!("{:>4} │ ", i + 1);
                                    draw_str(
                                        screen,
                                        inner_x,
                                        y,
                                        &line_nr_str,
                                        &linenr_style,
                                        prev_end - 1,
                                    );
                                    let text_x = inner_x + line_nr_str.len() as u16;
                                    let max_code_x = prev_end.saturating_sub(1);

                                    let line_spans = highlighted_preview
                                        .iter()
                                        .find(|(l, _)| *l == i)
                                        .map_or(&[][..], |(_, spans)| spans.as_slice());

                                    for (char_idx, ch) in line.chars().enumerate() {
                                        let char_col = text_x + char_idx as u16;
                                        if char_col >= max_code_x {
                                            break;
                                        }

                                        let mut char_style = text_style;
                                        if let Some(span) = line_spans
                                            .iter()
                                            .find(|s| char_idx >= s.start && char_idx < s.end)
                                        {
                                            let scope_style = style_for_scope(theme, span.scope());
                                            char_style = merge_style(char_style, scope_style);
                                        }

                                        screen.set(
                                            point(char_col, y),
                                            &char_style,
                                            &ch.to_string(),
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                let no_prev = "<No preview>";
                let px = inner_x + (inner_w.saturating_sub(no_prev.len() as u16)) / 2;
                let py = 1 + inner_h / 2;
                draw_str(screen, px, py, no_prev, &inactive_style, prev_end);
            }
        }

        let cursor_col = (4 + p.query.chars().count() as u16).min(picker_width.saturating_sub(2));
        cursor_render_info {
            col: cursor_col,
            row: 1,
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

/// Wraps text into lines that do not exceed `max_width`.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= max_width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Finds the character indices in `text` that match the subsequence `query`.
fn find_subsequence_matches(text: &str, query: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let q_chars: Vec<char> = query.to_lowercase().chars().collect();
    let t_chars: Vec<char> = text.to_lowercase().chars().collect();
    let mut indices = Vec::with_capacity(q_chars.len());
    let mut q_idx = 0;
    for (t_idx, &t_ch) in t_chars.iter().enumerate() {
        if q_idx < q_chars.len() && t_ch == q_chars[q_idx] {
            indices.push(t_idx);
            q_idx += 1;
        }
    }
    if q_idx == q_chars.len() {
        indices
    } else {
        Vec::new()
    }
}
