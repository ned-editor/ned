//! Gutter layout, line number formatting, and Git diff indicator rendering.

use ned_config::{line_number_style, mode};
use ned_git::{diff_tag, hunk};
use ned_tui::geometry::point;
use ned_tui::output::style::Style;
use ned_tui::Screen;

/// Computes the dynamic width of the gutter for `line_count` matching Helix layout:
/// `[diagnostic (1)] [line_digits] [space (1)] [diff indicator (1)]`.
#[must_use]
pub fn calculate_gutter_width(line_count: usize) -> u16 {
    let digits = line_count.max(1).to_string().len().max(2);
    // LSP diagnostic symbol (1) + Line digits + Space separator (1) + Git diff indicator (1)
    (1 + digits + 1 + 1) as u16
}

/// Formats the line number string respecting relative, absolute, and insert mode rules matching Helix.
#[must_use]
pub fn format_line_number(
    line_idx: usize,
    active_idx: usize,
    style: line_number_style,
    width: usize,
    current_mode: mode,
) -> String {
    match style {
        line_number_style::none => " ".repeat(width),
        line_number_style::absolute => {
            format!("{:>width$}", line_idx + 1)
        }
        line_number_style::relative => {
            // Helix switches relative line numbers to absolute in insert mode
            if current_mode == mode::insert || line_idx == active_idx {
                format!("{:>width$}", line_idx + 1)
            } else {
                let diff = (line_idx as isize - active_idx as isize).unsigned_abs();
                format!("{diff:>width$}")
            }
        }
    }
}

/// Computes the diff indicator symbol and style matching editor layout.
///
/// Symbols:
/// - Added: `+` with `diff.plus` (green)
/// - Modified: `~` with `diff.delta` (yellow)
/// - Deleted: `-` with `diff.minus` (red)
#[must_use]
pub fn get_diff_indicator(
    hunks: &[hunk],
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
                    return ("+", plus_style);
                }
            }
            diff_tag::modified => {
                if count > 0 && line_idx >= start && line_idx < start + count {
                    return ("~", delta_style);
                }
            }
            diff_tag::deleted => {
                if line_idx == start || (start >= total_lines && line_idx + 1 == total_lines) {
                    return ("-", minus_style);
                }
            }
        }
    }
    (" ", gutter_style)
}

/// Paints the full gutter columns for a line row: `[diagnostic] [line-number] [space] [diff]`.
pub fn render_gutter_row(
    screen: &mut Screen,
    row: u16,
    gutter_width: u16,
    diag_sym: &'static str,
    diag_style: &Style,
    num_str: &str,
    num_style: &Style,
    diff_sym: &'static str,
    diff_style: &Style,
) {
    // Column 0: LSP diagnostic indicator
    screen.set(point(0, row), diag_style, diag_sym);

    // Columns 1..gutter_width-2: Line numbers
    for (i, ch) in num_str.chars().enumerate() {
        let col = (1 + i) as u16;
        let s = ch.to_string();
        screen.set(point(col, row), num_style, &s);
    }

    // Column gutter_width - 2: Space separator on left side of diff bar
    screen.set(point(gutter_width - 2, row), num_style, " ");

    // Column gutter_width - 1: Git diff indicator (adjacent to code area)
    screen.set(point(gutter_width - 1, row), diff_style, diff_sym);
}
