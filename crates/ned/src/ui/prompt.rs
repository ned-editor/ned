//! Command prompt rendering painting prefix, input text, completion menus, and doc popups.

use ned_config::cursor_shape;
use ned_core::editor;
use ned_tui::geometry::point;
use ned_tui::Screen;

use crate::prompt::prompt_state;
use crate::theme_bridge::{merge_style, style_for_scope};
use crate::ui::{cursor_render_info, draw_str, wrap_text};

/// Paints the prompt line, completion grid, and command documentation popup.
#[must_use]
pub fn render_prompt(
    screen: &mut Screen,
    ed: &editor,
    prompt: &prompt_state,
    width: u16,
    cmdline_row: u16,
) -> cursor_render_info {
    let theme = &ed.config.theme;
    let base_text_style = style_for_scope(theme, "ui.text");
    let base_bg_style = style_for_scope(theme, "ui.background");
    let default_style = merge_style(base_bg_style, base_text_style);
    let cmdline_style = default_style;

    for c in 0..width {
        screen.set(point(c, cmdline_row), &cmdline_style, " ");
    }

    let prompt_display = format!("{}{}", prompt.prefix, prompt.input);
    draw_str(screen, 0, cmdline_row, &prompt_display, &cmdline_style, width);

    if prompt.prefix == ":" {
        let completions = &prompt.completion;
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
            let offset = prompt
                .selection
                .map_or(0, |idx| (idx / items) * items);

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
                let is_sel = prompt.selection == Some(i);
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
        if let Some((cmd_name, doc_text)) = prompt.active_doc() {
            let max_box_width =
                ((base_width * 3) as usize).min((width as usize).saturating_sub(2));
            let inner_text_width = max_box_width.saturating_sub(4).max(10);
            let full_doc = format!("{cmd_name}: {doc_text}");
            let wrapped_lines = wrap_text(&full_doc, inner_text_width);

            let doc_box_width = ((wrapped_lines.iter().map(String::len).max().unwrap_or(20) + 4)
                as u16)
                .min(width)
                .max(25);
            let doc_box_height = (wrapped_lines.len() as u16 + 2)
                .min(cmdline_row.saturating_sub(completion_height));

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

    let cursor_col = (prompt.prefix.chars().count() + prompt.cursor).min(width as usize) as u16;
    cursor_render_info {
        col: cursor_col,
        row: cmdline_row,
        shape: cursor_shape::bar,
        visible: true,
    }
}
