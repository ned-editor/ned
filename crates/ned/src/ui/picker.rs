//! Modal split picker rendering candidate items, fuzzy match highlights, and live preview panes.

use ned_config::cursor_shape;
use ned_core::editor;
use ned_tui::geometry::point;
use ned_tui::Screen;

use crate::picker::{min_area_width_for_preview, picker_state, preview_content};
use crate::theme_bridge::{merge_style, style_for_scope};
use crate::ui::{cursor_render_info, draw_str, find_subsequence_matches};

/// Paints the modal picker overlay and live preview window onto the screen.
#[must_use]
pub fn render_picker(
    screen: &mut Screen,
    ed: &editor,
    picker: &picker_state,
    width: u16,
    height: u16,
) -> cursor_render_info {
    let theme = &ed.config.theme;
    let base_text_style = style_for_scope(theme, "ui.text");
    let base_bg_style = style_for_scope(theme, "ui.background");
    let default_style = merge_style(base_bg_style, base_text_style);

    let popup_style = merge_style(default_style, style_for_scope(theme, "ui.menu"));
    let popup_sel_style = merge_style(default_style, style_for_scope(theme, "ui.menu.selected"));
    let border_style = merge_style(popup_style, style_for_scope(theme, "ui.window"));
    let text_style = merge_style(default_style, style_for_scope(theme, "ui.text"));
    let linenr_style = merge_style(default_style, style_for_scope(theme, "ui.linenr"));
    let sep_style = merge_style(
        border_style,
        style_for_scope(theme, "ui.background.separator"),
    );
    let inactive_style = merge_style(default_style, style_for_scope(theme, "ui.text.inactive"));
    let match_style = merge_style(
        style_for_scope(theme, "special"),
        style_for_scope(theme, "ui.text.focus"),
    );

    let fits_beside = width > min_area_width_for_preview && picker.show_preview;
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
    let query_prompt = format!("> {}", picker.query);
    draw_str(
        screen,
        2,
        1,
        &query_prompt,
        &text_style,
        picker_width.saturating_sub(12),
    );

    let count_str = if let Some(err) = &picker.search_error {
        format!("Regex error: {err}")
    } else {
        format!("{}/{}", picker.filtered.len(), picker.items.len())
    };
    let count_col = (picker_width - 1).saturating_sub(count_str.len() as u16 + 1);
    if count_col > 4 + picker.query.chars().count() as u16 {
        let count_style = if picker.search_error.is_some() {
            merge_style(default_style, style_for_scope(theme, "diagnostic.error"))
        } else {
            text_style
        };
        draw_str(
            screen,
            count_col,
            1,
            &count_str,
            &count_style,
            picker_width - 1,
        );
    }

    // Picker separator line (row 2)
    screen.set(point(0, 2), &border_style, "├");
    screen.set(point(picker_width - 1, 2), &border_style, "┤");
    for c in 1..picker_width - 1 {
        screen.set(point(c, 2), &sep_style, "─");
    }

    // Picker items list (rows 3 .. height - 1)
    let list_height = height.saturating_sub(4) as usize;
    let scroll_offset = if picker.selected >= list_height {
        picker.selected - list_height + 1
    } else {
        0
    };

    if let Some(err) = &picker.search_error {
        if picker.filtered.is_empty() && list_height > 0 {
            let err_style = merge_style(default_style, style_for_scope(theme, "diagnostic.error"));
            draw_str(
                screen,
                2,
                3,
                &format!("Regex error: {err}"),
                &err_style,
                picker_width.saturating_sub(4),
            );
        }
    }

    for i in 0..list_height {
        let row_y = 3 + i as u16;
        screen.set(point(0, row_y), &border_style, "│");
        screen.set(point(picker_width - 1, row_y), &border_style, "│");

        let filter_idx = scroll_offset + i;
        if filter_idx < picker.filtered.len() {
            let item_idx = picker.filtered[filter_idx];
            let item_text = &picker.items[item_idx];
            let is_selected = filter_idx == picker.selected;
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
            let match_indices = find_subsequence_matches(item_text, &picker.query);

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
                screen.set(point(char_x, row_y), &char_style, ch.to_string());
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

        if let Some(preview) = picker.get_preview() {
            if let Some(placeholder) = preview.placeholder() {
                let px = inner_x + (inner_w.saturating_sub(placeholder.len() as u16)) / 2;
                let py = 1 + inner_h / 2;
                draw_str(screen, px, py, placeholder, &inactive_style, prev_end);
            } else {
                match preview {
                    preview_content::directory(entries) => {
                        for (i, (name, is_dir)) in entries.iter().take(inner_h as usize).enumerate()
                        {
                            let y = 1 + i as u16;
                            let display = if *is_dir {
                                format!("{name}/")
                            } else {
                                name.clone()
                            };
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
                            let px = inner_x + (inner_w.saturating_sub(empty_msg.len() as u16)) / 2;
                            let py = 1 + inner_h / 2;
                            draw_str(screen, px, py, empty_msg, &inactive_style, prev_end);
                        } else {
                            let preview_path = picker.selected_item().map(|item| {
                                if let Some(root) = &picker.root_dir {
                                    root.join(item)
                                } else {
                                    std::path::PathBuf::from(item)
                                }
                            });

                            let lang_cfg = preview_path
                                .as_deref()
                                .and_then(|path| ed.config.languages.detect_by_path(path));
                            let grammar = lang_cfg.and_then(|cfg| {
                                ned_core::load_grammar(cfg.grammar.as_deref().unwrap_or(&cfg.name))
                            });

                            let highlighted_preview = if let Some(grammar) = grammar {
                                let mut highlighter =
                                    ned_tree::syntax_highlighter::new(Some(grammar));
                                let preview_text = lines.join("\n");
                                let rope_buf = ned_rope::rope::from_string(&preview_text);
                                highlighter.highlight_viewport(
                                    &rope_buf,
                                    0,
                                    lines.len().min(inner_h as usize),
                                )
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

                                    screen.set(point(char_col, y), &char_style, ch.to_string());
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

    let cursor_col = (4 + picker.query.chars().count() as u16).min(picker_width.saturating_sub(2));
    cursor_render_info {
        col: cursor_col,
        row: 1,
        shape: cursor_shape::bar,
        visible: true,
    }
}
