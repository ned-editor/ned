//! Selection overlay styling for primary and secondary text selections.

use ned_text::{range, selection};
use ned_tui::output::style::Style;

use crate::theme_bridge::merge_style;

/// Applies primary or secondary selection styling if `global_char_idx` falls within any active selection.
pub fn apply_selection_style(
    selections: &selection,
    primary_range: range,
    global_char_idx: usize,
    base_style: Style,
    sel_primary_style: Style,
    sel_secondary_style: Style,
) -> Style {
    for sel_range in selections {
        let is_primary = *sel_range == primary_range;
        if sel_range.contains(global_char_idx) {
            let sel_overlay = if is_primary {
                sel_primary_style
            } else {
                sel_secondary_style
            };
            return merge_style(base_style, sel_overlay);
        }
    }
    base_style
}
