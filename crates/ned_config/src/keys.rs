//! Modal keymaps, key event parsing, and editor action definitions.
//!
//! Replicates Helix modal editing key semantics (Normal, Insert, Select modes,
//! `g` goto sub-layer, `[` and `]` brackets) using Ned's native types.

use std::collections::HashMap;
pub mod actions;
pub mod key_types;
pub mod parser;
pub mod trie;

pub use actions::action;
pub use key_types::{key_code, key_event, key_modifiers, mode};
pub use parser::parse_keymaps_from_toml;
pub use trie::{key_trie, key_trie_node};

/// Constructs the complete set of default Helix-equivalent modal keymaps.
#[must_use]
pub fn default_keymaps() -> HashMap<mode, key_trie_node> {
    let mut keymaps = HashMap::new();

    // ---------------- Normal mode ----------------
    let mut normal = key_trie_node::new("Normal mode");

    // Movement
    normal.insert("h", action::move_char_left);
    normal.insert("left", action::move_char_left);
    normal.insert("j", action::move_line_down);
    normal.insert("down", action::move_line_down);
    normal.insert("k", action::move_line_up);
    normal.insert("up", action::move_line_up);
    normal.insert("l", action::move_char_right);
    normal.insert("right", action::move_char_right);

    normal.insert("w", action::move_next_word_start);
    normal.insert("b", action::move_prev_word_start);
    normal.insert("e", action::move_next_word_end);
    normal.insert("W", action::move_next_long_word_start);
    normal.insert("B", action::move_prev_long_word_start);
    normal.insert("E", action::move_next_long_word_end);

    normal.insert("home", action::goto_line_start);
    normal.insert("end", action::goto_line_end);

    // Modes & insertions
    normal.insert("i", action::insert_mode);
    normal.insert("I", action::insert_at_line_start);
    normal.insert("a", action::append_mode);
    normal.insert("A", action::insert_at_line_end);
    normal.insert("o", action::open_below);
    normal.insert("O", action::open_above);
    normal.insert("v", action::select_mode);
    normal.insert(":", action::command_mode);

    // Deletions & changes
    normal.insert("d", action::delete_selection);
    normal.insert("A-d", action::delete_selection_noyank);
    normal.insert("c", action::change_selection);
    normal.insert("A-c", action::change_selection_noyank);
    normal.insert("r", action::replace);
    normal.insert("y", action::yank);
    normal.insert("p", action::paste_after);
    normal.insert("P", action::paste_before);
    normal.insert("u", action::undo);
    normal.insert("U", action::redo);
    normal.insert("~", action::switch_case);
    normal.insert("`", action::switch_to_lowercase);
    normal.insert("A-`", action::switch_to_uppercase);
    normal.insert(">", action::indent);
    normal.insert("<", action::unindent);

    // Selections
    normal.insert("x", action::extend_line_below);
    normal.insert("X", action::extend_to_line_bounds);
    normal.insert("%", action::select_all);
    normal.insert(";", action::collapse_selection);
    normal.insert("A-;", action::flip_selections);
    normal.insert(",", action::keep_primary_selection);

    // Search
    normal.insert("/", action::search);
    normal.insert("?", action::rsearch);
    normal.insert("n", action::search_next);
    normal.insert("N", action::search_prev);

    // LSP
    normal.insert("K", action::hover);

    // 'g' goto sub-layer
    let mut goto = key_trie_node::new("Goto");
    goto.insert("g", action::goto_file_start);
    goto.insert("e", action::goto_file_end);
    goto.insert("h", action::goto_line_start);
    goto.insert("l", action::goto_line_end);
    goto.insert("s", action::goto_first_nonwhitespace);
    goto.insert("d", action::goto_definition);
    goto.insert("r", action::goto_reference);
    goto.insert("k", action::move_line_up);
    goto.insert("j", action::move_line_down);
    normal.insert_node("g", goto);

    // '[' and ']' diagnostic brackets
    let mut left_bracket = key_trie_node::new("Left bracket");
    left_bracket.insert("d", action::goto_prev_diag);
    normal.insert_node("[", left_bracket);

    let mut right_bracket = key_trie_node::new("Right bracket");
    right_bracket.insert("d", action::goto_next_diag);
    normal.insert_node("]", right_bracket);

    // Space leader keymap
    let mut space = key_trie_node::new("Space");
    space.insert("f", action::file_picker);
    space.insert("b", action::buffer_picker);
    space.insert("d", action::diagnostics_picker);
    space.insert("k", action::hover);
    space.insert("y", action::yank);
    normal.insert_node("space", space);

    keymaps.insert(mode::normal, normal);

    // ---------------- Insert mode ----------------
    let mut insert = key_trie_node::new("Insert mode");
    insert.insert("esc", action::normal_mode);
    insert.insert("ret", action::insert_newline);
    insert.insert("tab", action::insert_tab);
    insert.insert("backspace", action::delete_char_backward);
    insert.insert("C-w", action::delete_word_backward);
    keymaps.insert(mode::insert, insert);

    // ---------------- Select mode ----------------
    // In select mode, motions extend selections rather than moving them.
    let mut select = key_trie_node::new("Select mode");
    select.insert("h", action::move_char_left);
    select.insert("j", action::move_line_down);
    select.insert("k", action::move_line_up);
    select.insert("l", action::move_char_right);
    select.insert("w", action::move_next_word_start);
    select.insert("b", action::move_prev_word_start);
    select.insert("e", action::move_next_word_end);
    select.insert("v", action::normal_mode);
    select.insert("esc", action::normal_mode);
    select.insert(";", action::collapse_selection);
    select.insert("d", action::delete_selection);
    select.insert("c", action::change_selection);
    select.insert("y", action::yank);
    keymaps.insert(mode::select, select);

    keymaps
}
