//! Default Helix-equivalent modal keymaps and prefix key bindings.

use std::collections::HashMap;

use super::actions::action;
use super::key_types::mode;
use super::trie::key_trie_node;

/// Constructs the complete set of default Helix-equivalent modal keymaps.
///
/// This is a large data-driven function inserting hundreds of key bindings;
/// splitting it would fragment the keymap table without improving clarity,
/// so the line-limit lint is allowed here.
#[allow(clippy::too_many_lines)]
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
    normal.insert("G", action::goto_line);
    normal.insert("C-d", action::scroll_down);
    normal.insert("C-u", action::scroll_up);
    normal.insert("pagedown", action::scroll_down);
    normal.insert("pageup", action::scroll_up);
    normal.insert("C-f", action::scroll_down);
    normal.insert("C-b", action::scroll_up);

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
    normal.insert("R", action::replace_with_yanked);
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
    normal.insert("C-c", action::toggle_line_comment);

    // Selections
    normal.insert("x", action::extend_line_below);
    normal.insert("X", action::extend_to_line_bounds);
    normal.insert("A-x", action::shrink_to_line_bounds);
    normal.insert("%", action::select_all);
    normal.insert("s", action::select_regex);
    normal.insert("S", action::split_selection);
    normal.insert("A-s", action::split_selection_on_newline);
    normal.insert("A-_", action::merge_consecutive_selections);
    normal.insert("A-minus", action::merge_consecutive_selections);
    normal.insert("&", action::align_selections);
    normal.insert("_", action::trim_selections);
    normal.insert(";", action::collapse_selection);
    normal.insert("A-;", action::flip_selections);
    normal.insert("A-:", action::ensure_selection_forward);
    normal.insert(",", action::keep_primary_selection);
    normal.insert("A-,", action::remove_primary_selection);
    normal.insert("C", action::copy_selection_on_next_line);
    normal.insert("A-C", action::copy_selection_on_prev_line);
    normal.insert(")", action::rotate_selections_forward);
    normal.insert("(", action::rotate_selections_backward);
    normal.insert("A-)", action::rotate_selection_contents_forward);
    normal.insert("A-(", action::rotate_selection_contents_backward);
    normal.insert("J", action::join_selections);
    normal.insert("A-J", action::join_selections_space);
    normal.insert("K", action::keep_matching_selections);
    normal.insert("A-K", action::remove_matching_selections);

    // Syntax tree navigation
    normal.insert("A-o", action::expand_selection);
    normal.insert("A-i", action::shrink_selection);
    normal.insert("A-p", action::select_prev_sibling);
    normal.insert("A-n", action::select_next_sibling);

    // Search
    normal.insert("/", action::search);
    normal.insert("?", action::rsearch);
    normal.insert("n", action::search_next);
    normal.insert("N", action::search_prev);

    // 'g' goto sub-layer
    let mut goto = key_trie_node::new("Goto");
    goto.insert("g", action::goto_file_start);
    goto.insert("e", action::goto_file_end);
    goto.insert("h", action::goto_line_start);
    goto.insert("l", action::goto_line_end);
    goto.insert("s", action::goto_first_nonwhitespace);
    goto.insert("t", action::goto_window_top);
    goto.insert("c", action::goto_window_center);
    goto.insert("b", action::goto_window_bottom);
    goto.insert("n", action::goto_next_buffer);
    goto.insert("p", action::goto_previous_buffer);
    goto.insert("d", action::goto_definition);
    goto.insert("r", action::goto_reference);
    goto.insert("k", action::move_line_up);
    goto.insert("j", action::move_line_down);
    normal.insert_node("g", goto.clone());

    // 'z' and 'Z' view sub-layer
    let mut view = key_trie_node::new("View");
    view.insert("c", action::align_view_center);
    view.insert("z", action::align_view_center);
    view.insert("m", action::align_view_center);
    view.insert("t", action::align_view_top);
    view.insert("b", action::align_view_bottom);
    view.insert("j", action::scroll_down);
    view.insert("down", action::scroll_down);
    view.insert("k", action::scroll_up);
    view.insert("up", action::scroll_up);
    view.insert("C-d", action::scroll_down);
    view.insert("C-u", action::scroll_up);
    view.insert("C-f", action::scroll_down);
    view.insert("pagedown", action::scroll_down);
    view.insert("C-b", action::scroll_up);
    view.insert("pageup", action::scroll_up);
    normal.insert_node("z", view.clone());
    normal.insert_node("Z", view.clone());

    // 'm' match sub-layer
    let mut match_trie = key_trie_node::new("Match");
    match_trie.insert("m", action::match_brackets);
    match_trie.insert("a", action::select_textobject_around);
    match_trie.insert("i", action::select_textobject_inside);
    match_trie.insert("s", action::surround_add);
    match_trie.insert("r", action::surround_replace);
    match_trie.insert("d", action::surround_delete);
    normal.insert_node("m", match_trie.clone());

    // 'C-w' window sub-layer
    let mut window = key_trie_node::new("Window");
    window.insert("v", action::vsplit);
    window.insert("C-v", action::vsplit);
    window.insert("s", action::hsplit);
    window.insert("C-s", action::hsplit);
    window.insert("w", action::rotate_view);
    window.insert("C-w", action::rotate_view);
    window.insert("q", action::wclose);
    window.insert("C-q", action::wclose);
    window.insert("o", action::wonly);
    window.insert("C-o", action::wonly);
    window.insert("h", action::jump_view_left);
    window.insert("C-h", action::jump_view_left);
    window.insert("left", action::jump_view_left);
    window.insert("j", action::jump_view_down);
    window.insert("C-j", action::jump_view_down);
    window.insert("down", action::jump_view_down);
    window.insert("k", action::jump_view_up);
    window.insert("C-k", action::jump_view_up);
    window.insert("up", action::jump_view_up);
    window.insert("l", action::jump_view_right);
    window.insert("C-l", action::jump_view_right);
    window.insert("right", action::jump_view_right);
    normal.insert_node("C-w", window.clone());

    // '[' and ']' bracket navigation (diagnostics and git diff hunks)
    let mut left_bracket = key_trie_node::new("Left bracket");
    left_bracket.insert("d", action::goto_prev_diag);
    left_bracket.insert("g", action::goto_prev_hunk);
    left_bracket.insert("h", action::goto_prev_hunk);
    normal.insert_node("[", left_bracket.clone());

    let mut right_bracket = key_trie_node::new("Right bracket");
    right_bracket.insert("d", action::goto_next_diag);
    right_bracket.insert("g", action::goto_next_hunk);
    right_bracket.insert("h", action::goto_next_hunk);
    normal.insert_node("]", right_bracket.clone());

    // Space leader keymap
    let mut space = key_trie_node::new("Space");
    space.insert("f", action::file_picker);
    space.insert("F", action::file_picker);
    space.insert("b", action::buffer_picker);
    space.insert("d", action::diagnostics_picker);
    space.insert("D", action::diagnostics_picker);
    space.insert("/", action::global_search);
    space.insert("k", action::hover);
    space.insert("y", action::yank);
    space.insert("p", action::paste_after);
    space.insert("P", action::paste_before);
    space.insert_node("w", window.clone());
    normal.insert_node("space", space.clone());

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
    select.insert("left", action::move_char_left);
    select.insert("j", action::move_line_down);
    select.insert("down", action::move_line_down);
    select.insert("k", action::move_line_up);
    select.insert("up", action::move_line_up);
    select.insert("l", action::move_char_right);
    select.insert("right", action::move_char_right);

    select.insert("w", action::move_next_word_start);
    select.insert("b", action::move_prev_word_start);
    select.insert("e", action::move_next_word_end);
    select.insert("W", action::move_next_long_word_start);
    select.insert("B", action::move_prev_long_word_start);
    select.insert("E", action::move_next_long_word_end);

    select.insert("home", action::goto_line_start);
    select.insert("end", action::goto_line_end);
    select.insert("G", action::goto_line);
    select.insert("C-d", action::scroll_down);
    select.insert("C-u", action::scroll_up);
    select.insert("pagedown", action::scroll_down);
    select.insert("pageup", action::scroll_up);
    select.insert("C-f", action::scroll_down);
    select.insert("C-b", action::scroll_up);

    select.insert("v", action::normal_mode);
    select.insert("esc", action::normal_mode);
    select.insert(":", action::command_mode);

    select.insert("i", action::insert_mode);
    select.insert("I", action::insert_at_line_start);
    select.insert("a", action::append_mode);
    select.insert("A", action::insert_at_line_end);
    select.insert("o", action::open_below);
    select.insert("O", action::open_above);

    select.insert("d", action::delete_selection);
    select.insert("A-d", action::delete_selection_noyank);
    select.insert("c", action::change_selection);
    select.insert("A-c", action::change_selection_noyank);
    select.insert("r", action::replace);
    select.insert("R", action::replace_with_yanked);
    select.insert("y", action::yank);
    select.insert("p", action::paste_after);
    select.insert("P", action::paste_before);
    select.insert("u", action::undo);
    select.insert("U", action::redo);
    select.insert("~", action::switch_case);
    select.insert("`", action::switch_to_lowercase);
    select.insert("A-`", action::switch_to_uppercase);
    select.insert(">", action::indent);
    select.insert("<", action::unindent);
    select.insert("C-c", action::toggle_line_comment);

    select.insert("x", action::extend_line_below);
    select.insert("X", action::extend_to_line_bounds);
    select.insert("A-x", action::shrink_to_line_bounds);
    select.insert("%", action::select_all);
    select.insert("s", action::select_regex);
    select.insert("S", action::split_selection);
    select.insert("A-s", action::split_selection_on_newline);
    select.insert("A-_", action::merge_consecutive_selections);
    select.insert("A-minus", action::merge_consecutive_selections);
    select.insert("&", action::align_selections);
    select.insert("_", action::trim_selections);
    select.insert(";", action::collapse_selection);
    select.insert("A-;", action::flip_selections);
    select.insert("A-:", action::ensure_selection_forward);
    select.insert(",", action::keep_primary_selection);
    select.insert("A-,", action::remove_primary_selection);
    select.insert("C", action::copy_selection_on_next_line);
    select.insert("A-C", action::copy_selection_on_prev_line);
    select.insert(")", action::rotate_selections_forward);
    select.insert("(", action::rotate_selections_backward);
    select.insert("A-)", action::rotate_selection_contents_forward);
    select.insert("A-(", action::rotate_selection_contents_backward);
    select.insert("J", action::join_selections);
    select.insert("A-J", action::join_selections_space);
    select.insert("K", action::keep_matching_selections);
    select.insert("A-K", action::remove_matching_selections);

    // Syntax tree navigation
    select.insert("A-o", action::expand_selection);
    select.insert("A-i", action::shrink_selection);
    select.insert("A-p", action::select_prev_sibling);
    select.insert("A-n", action::select_next_sibling);

    select.insert("/", action::search);
    select.insert("?", action::rsearch);
    select.insert("n", action::search_next);
    select.insert("N", action::search_prev);
    select.insert_node("g", goto);
    select.insert_node("z", view.clone());
    select.insert_node("Z", view);
    select.insert_node("m", match_trie);
    select.insert_node("C-w", window);
    select.insert_node("[", left_bracket);
    select.insert_node("]", right_bracket);
    select.insert_node("space", space);
    keymaps.insert(mode::select, select);

    keymaps
}
