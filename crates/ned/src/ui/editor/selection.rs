//! Selection overlay styling for primary and secondary text selections.

use ned_rope::rope;
use ned_text::{range, selection};
use ned_tui::output::style::Style;

use crate::theme_bridge::merge_style;

/// Applies primary or secondary selection styling if `global_char_idx` falls within any active selection.
///
/// The eight arguments represent the selection state, target buffer, cursor position, base style,
/// and three overlay styles; grouping them would add indirection without improving clarity, so the
/// argument-count lint is allowed here.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn apply_selection_style(
    selections: &selection,
    primary_range: range,
    buffer: &rope,
    global_char_idx: usize,
    base_style: Style,
    sel_primary_style: Style,
    sel_secondary_style: Style,
    cursor_secondary_style: Style,
) -> Style {
    for sel_range in selections {
        let is_primary = *sel_range == primary_range;
        let effective_range = sel_range.min_width_1(buffer);
        if effective_range.contains(global_char_idx) {
            let sel_overlay = if is_primary {
                sel_primary_style
            } else if global_char_idx == sel_range.cursor(buffer) {
                cursor_secondary_style
            } else {
                sel_secondary_style
            };
            return merge_style(base_style, sel_overlay);
        }
    }
    base_style
}
