//! User interface rendering coordination painting editor, statusline, prompt, picker, and which-key popups.

pub mod editor;
pub mod info;
pub mod picker;
pub mod prompt;
pub mod statusline;

pub use editor::calculate_gutter_width;
use ned_config::cursor_shape;
use ned_core::editor as editor_state;
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

/// Paints the entire editor frame to the double-buffered `Screen`.
///
/// Returns cursor positioning information for post-paint terminal synchronization.
///
/// # Panics
///
/// Panics if the active document or active view does not exist in the editor.
pub fn render_frame(
    screen: &mut Screen,
    ed: &mut editor_state,
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
    let total_lines = ed
        .documents
        .get(&doc_id)
        .map_or(1, |d| d.text().line_count().max(1));
    let gutter_width = calculate_gutter_width(total_lines);

    // 1. Render main editor text area & gutter
    editor::render_editor(screen, ed, width, editor_height, gutter_width);

    // 2. Render statusline
    statusline::render_statusline(screen, ed, width, statusline_row);

    // 3. Clear and render command line / status message base
    let theme = &ed.config.theme;
    let base_text_style = style_for_scope(theme, "ui.text");
    let base_bg_style = style_for_scope(theme, "ui.background");
    let default_style = merge_style(base_bg_style, base_text_style);
    let cmdline_style = default_style;
    for c in 0..width {
        screen.set(point(c, cmdline_row), &cmdline_style, " ");
    }

    // 4. Overlays: prompt or picker or editor status
    if let Some(p) = prompt {
        prompt::render_prompt(screen, ed, p, width, cmdline_row)
    } else if let Some(p) = picker {
        picker::render_picker(screen, ed, p, width, height)
    } else {
        if let Some(msg) = &ed.status_message {
            draw_str(screen, 0, cmdline_row, msg, &cmdline_style, width);
        }

        // Render which-key autoinfo popup if a prefix key (Space, g, [, ]) is active
        if let Some(ref node) = ed.key_prefix {
            info::render_autoinfo(screen, ed, node, width, height, statusline_row);
        }

        let doc = ed
            .documents
            .get(&ed.active_doc_id)
            .expect("active document must exist");
        let view = ed
            .views
            .get(&ed.active_view_id)
            .expect("active view must exist");

        let cursor_info = editor::cursor::compute_cursor_info(
            view,
            doc,
            gutter_width,
            ed.mode,
            &ed.config.editor.cursor_shape,
        );

        if let Some(ref hover_text) = ed.hover_popup {
            info::render_hover_popup(
                screen,
                ed,
                hover_text,
                width,
                statusline_row,
                cursor_info.col,
                cursor_info.row,
            );
        }

        cursor_info
    }
}

/// Helper drawing a string horizontally onto `Screen` up to `max_col`.
pub fn draw_str(
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

/// Wraps text into lines that do not exceed `max_width`.
#[must_use]
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
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
#[must_use]
pub fn find_subsequence_matches(text: &str, query: &str) -> Vec<usize> {
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
