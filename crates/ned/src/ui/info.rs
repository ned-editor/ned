//! Autoinfo which-key panel rendering keybinding hints for prefix keys matching 1:1 Helix layout and theme scopes.

use ned_config::{key_trie, key_trie_node};
use ned_core::editor;
use ned_tui::geometry::point;
use ned_tui::Screen;

use crate::theme_bridge::{merge_style, style_for_scope};
use crate::ui::{draw_str, wrap_text};

/// Renders the floating which-key autoinfo popup in the bottom right corner.
///
/// Uses standard Helix theme scopes (`ui.popup`, `ui.window`, `ui.text.focus`, `special`, `ui.text`)
/// without any hardcoded colors, ensuring complete fidelity across all user themes.
pub fn render_autoinfo(
    screen: &mut Screen,
    ed: &editor,
    node: &key_trie_node,
    width: u16,
    height: u16,
    statusline_row: u16,
) {
    if node.map.is_empty() || width < 25 || height < 6 {
        return;
    }

    let theme = &ed.config.theme;
    let base_text_style = style_for_scope(theme, "ui.text");
    let base_bg_style = style_for_scope(theme, "ui.background");
    let default_style = merge_style(base_bg_style, base_text_style);

    // Dynamic theme scope resolution (no hardcoded colors)
    let popup_style = merge_style(default_style, style_for_scope(theme, "ui.popup"));
    let border_style = merge_style(popup_style, style_for_scope(theme, "ui.window"));
    let title_style = merge_style(border_style, style_for_scope(theme, "ui.text.focus"));
    let key_style = merge_style(popup_style, style_for_scope(theme, "special"));
    let desc_style = merge_style(popup_style, style_for_scope(theme, "ui.text"));

    // Collect and format entries sorted alphabetically by key
    let mut entries: Vec<(String, String)> = node
        .map
        .iter()
        .map(|(key, target)| {
            let key_str = key.to_string();
            let desc_str = match target {
                key_trie::action(act) => format!("{act:?}"),
                key_trie::node(sub) => sub.label.clone(),
                key_trie::sequence(_) => "sequence".to_string(),
            };
            (key_str, desc_str)
        })
        .collect();

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let max_avail_rows = (statusline_row.saturating_sub(3) as usize).clamp(3, 16);

    // Multi-column layout matching Helix when items exceed available vertical space
    let cols = if entries.len() > max_avail_rows {
        (entries.len().div_ceil(max_avail_rows)).min(3)
    } else {
        1
    };

    let rows_per_col = entries.len().div_ceil(cols);
    let popup_rows = (rows_per_col as u16).min(statusline_row.saturating_sub(2));

    let max_key_len = entries.iter().map(|(k, _)| k.len()).max().unwrap_or(1).max(2);
    let max_desc_len = entries.iter().map(|(_, d)| d.len()).max().unwrap_or(8);
    let col_content_width = (max_key_len + 2 + max_desc_len) as u16;
    let col_spacing = 3u16;

    let inner_width = (cols as u16) * col_content_width + (cols as u16).saturating_sub(1) * col_spacing;
    let title = format!(" {} ", node.label);
    let popup_width = (inner_width + 4)
        .max(title.len() as u16 + 4)
        .min(width.saturating_sub(2));
    let popup_height = popup_rows + 2;

    let start_col = width.saturating_sub(popup_width).saturating_sub(1);
    let start_row = statusline_row.saturating_sub(popup_height);
    let end_col = start_col + popup_width.saturating_sub(1);
    let end_row = start_row + popup_height.saturating_sub(1);

    // 1. Clear popup background
    for r in start_row..=end_row {
        for c in start_col..=end_col {
            screen.set(point(c, r), &popup_style, " ");
        }
    }

    // 2. Render borders
    screen.set(point(start_col, start_row), &border_style, "┌");
    screen.set(point(end_col, start_row), &border_style, "┐");
    screen.set(point(start_col, end_row), &border_style, "└");
    screen.set(point(end_col, end_row), &border_style, "┘");

    for c in start_col + 1..end_col {
        screen.set(point(c, start_row), &border_style, "─");
        screen.set(point(c, end_row), &border_style, "─");
    }
    for r in start_row + 1..end_row {
        screen.set(point(start_col, r), &border_style, "│");
        screen.set(point(end_col, r), &border_style, "│");
    }

    // 3. Render title in top border
    let title_x = start_col + 2;
    draw_str(screen, title_x, start_row, &title, &title_style, end_col - 1);

    // 4. Render items in aligned columns
    for (idx, (key_str, desc_str)) in entries.iter().enumerate() {
        let col_idx = idx / rows_per_col;
        let row_in_col = idx % rows_per_col;

        if col_idx >= cols || row_in_col as u16 >= popup_rows {
            continue;
        }

        let r = start_row + 1 + row_in_col as u16;
        let col_x = start_col + 2 + (col_idx as u16) * (col_content_width + col_spacing);

        if col_x + col_content_width >= end_col {
            continue;
        }

        // Render key in theme `special` scope
        let key_display = format!("{key_str:>max_key_len$}");
        draw_str(screen, col_x, r, &key_display, &key_style, end_col - 1);

        // Render separator
        draw_str(screen, col_x + max_key_len as u16, r, "  ", &desc_style, end_col - 1);

        // Render action description in theme `ui.text` scope
        let desc_x = col_x + max_key_len as u16 + 2;
        draw_str(screen, desc_x, r, desc_str, &desc_style, end_col - 1);
    }
}

/// Renders the floating LSP hover documentation popup near the cursor (matching Helix).
pub fn render_hover_popup(
    screen: &mut Screen,
    ed: &editor,
    content: &str,
    width: u16,
    statusline_row: u16,
    cursor_col: u16,
    cursor_row: u16,
) {
    if content.is_empty() || width < 25 || statusline_row < 5 {
        return;
    }

    let theme = &ed.config.theme;
    let base_text_style = style_for_scope(theme, "ui.text");
    let base_bg_style = style_for_scope(theme, "ui.background");
    let default_style = merge_style(base_bg_style, base_text_style);

    let popup_style = merge_style(default_style, style_for_scope(theme, "ui.popup"));
    let border_style = merge_style(popup_style, style_for_scope(theme, "ui.window"));
    let title_style = merge_style(border_style, style_for_scope(theme, "ui.text.focus"));
    let text_style = merge_style(popup_style, style_for_scope(theme, "ui.text"));

    let max_inner_w = (width.saturating_sub(6) as usize).clamp(20, 76);
    let mut wrapped_lines: Vec<String> = Vec::new();
    for raw_line in content.lines() {
        let trimmed = raw_line.trim_end();
        if trimmed.is_empty() {
            wrapped_lines.push(String::new());
        } else {
            wrapped_lines.extend(wrap_text(trimmed, max_inner_w));
        }
    }
    if wrapped_lines.is_empty() {
        return;
    }

    let max_line_len = wrapped_lines.iter().map(|l| l.chars().count()).max().unwrap_or(10);
    let popup_width = (max_line_len as u16 + 4).clamp(20, width.saturating_sub(2));
    let max_popup_h = statusline_row.min(18);
    let popup_height = (wrapped_lines.len() as u16 + 2).clamp(3, max_popup_h);

    let start_col = if cursor_col + popup_width < width {
        cursor_col
    } else {
        width.saturating_sub(popup_width).saturating_sub(1)
    };

    let start_row = if cursor_row >= popup_height {
        cursor_row - popup_height
    } else if cursor_row + 1 + popup_height <= statusline_row {
        cursor_row + 1
    } else {
        0
    };

    let end_col = start_col + popup_width.saturating_sub(1);
    let end_row = start_row + popup_height.saturating_sub(1);

    for r in start_row..=end_row {
        for c in start_col..=end_col {
            screen.set(point(c, r), &popup_style, " ");
        }
    }

    screen.set(point(start_col, start_row), &border_style, "┌");
    screen.set(point(end_col, start_row), &border_style, "┐");
    screen.set(point(start_col, end_row), &border_style, "└");
    screen.set(point(end_col, end_row), &border_style, "┘");

    for c in start_col + 1..end_col {
        screen.set(point(c, start_row), &border_style, "─");
        screen.set(point(c, end_row), &border_style, "─");
    }
    for r in start_row + 1..end_row {
        screen.set(point(start_col, r), &border_style, "│");
        screen.set(point(end_col, r), &border_style, "│");
    }

    draw_str(screen, start_col + 2, start_row, " info ", &title_style, end_col - 1);

    let max_lines = popup_height.saturating_sub(2) as usize;
    for (i, line) in wrapped_lines.iter().take(max_lines).enumerate() {
        draw_str(
            screen,
            start_col + 2,
            start_row + 1 + i as u16,
            line,
            &text_style,
            end_col - 1,
        );
    }
}
