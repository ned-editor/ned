//! Executable editor actions triggered by modal key combinations or typed commands.

use serde::{Deserialize, Serialize};

/// An executable editor action triggered by keybindings or commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum action {
    // --- Cursor motions ---
    /// Move cursor left one grapheme (`h`).
    move_char_left,
    /// Move cursor right one grapheme (`l`).
    move_char_right,
    /// Move cursor line up (`k`).
    move_line_up,
    /// Move cursor line down (`j`).
    move_line_down,
    /// Move cursor to next word start (`w`).
    move_next_word_start,
    /// Move cursor to previous word start (`b`).
    move_prev_word_start,
    /// Move cursor to next word end (`e`).
    move_next_word_end,
    /// Move cursor to next long word start (`W`).
    move_next_long_word_start,
    /// Move cursor to previous long word start (`B`).
    move_prev_long_word_start,
    /// Move cursor to next long word end (`E`).
    move_next_long_word_end,
    /// Move cursor to start of line (`gh` or `home`).
    goto_line_start,
    /// Move cursor to end of line (`gl` or `end`).
    goto_line_end,
    /// Move cursor to first non-whitespace character of line (`gs`).
    goto_first_nonwhitespace,
    /// Move cursor to start of document (`gg`).
    goto_file_start,
    /// Move cursor to end of document (`ge`).
    goto_file_end,
    /// Prompt or jump to line number (`G`).
    goto_line,
    /// Move cursor to top of window (`gt`).
    goto_window_top,
    /// Move cursor to center of window (`gc`).
    goto_window_center,
    /// Move cursor to bottom of window (`gb`).
    goto_window_bottom,
    /// Switch to next open buffer (`gn`).
    goto_next_buffer,
    /// Switch to previous open buffer (`gp`).
    goto_previous_buffer,

    // --- Line selection ---
    /// Select current line or extend selection downward (`x`).
    extend_line_below,
    /// Extend selection to full line bounds (`X`).
    extend_to_line_bounds,

    // --- Mode switching ---
    /// Switch to insert mode at cursor (`i`).
    insert_mode,
    /// Switch to insert mode after cursor (`a`).
    append_mode,
    /// Switch to insert mode at line start (`I`).
    insert_at_line_start,
    /// Switch to insert mode at line end (`A`).
    insert_at_line_end,
    /// Open a new line below and enter insert mode (`o`).
    open_below,
    /// Open a new line above and enter insert mode (`O`).
    open_above,
    /// Switch to select mode (`v`).
    select_mode,
    /// Return to normal mode (`esc`).
    normal_mode,
    /// Enter command line mode (`:`).
    command_mode,

    // --- Editing & deletions ---
    /// Delete selected text and yank to clipboard (`d`).
    delete_selection,
    /// Delete selected text without yanking (`A-d`).
    delete_selection_noyank,
    /// Change selection: delete text and enter insert mode (`c`).
    change_selection,
    /// Change selection without yanking (`A-c`).
    change_selection_noyank,
    /// Replace selected character (`r`).
    replace,
    /// Replace selection with yanked text (`R`).
    replace_with_yanked,
    /// Yank selected text to clipboard (`y`).
    yank,
    /// Paste clipboard text after cursor (`p`).
    paste_after,
    /// Paste clipboard text before cursor (`P`).
    paste_before,
    /// Undo previous change (`u`).
    undo,
    /// Redo previous undone change (`U`).
    redo,
    /// Invert letter casing of selection (`~`).
    switch_case,
    /// Convert selection to lowercase (`` ` ``).
    switch_to_lowercase,
    /// Convert selection to uppercase (`A-`).
    switch_to_uppercase,
    /// Indent selected lines (`>`).
    indent,
    /// Unindent selected lines (`<`).
    unindent,
    /// Toggle line comment on selected lines (`C-c`).
    toggle_line_comment,
    /// Toggle block comment on selection.
    toggle_block_comment,

    // --- Selection operations ---
    /// Select entire document (`%`).
    select_all,
    /// Select sub-matches of regex within current selections (`s`).
    select_regex,
    /// Split selections on regex matches (`S`).
    split_selection,
    /// Split selections on newlines (`A-s`).
    split_selection_on_newline,
    /// Merge consecutive selections (`A-minus` / `A-_`).
    merge_consecutive_selections,
    /// Align selections in columns (`&`).
    align_selections,
    /// Trim whitespace from selections (`_`).
    trim_selections,
    /// Collapse selections to single-character cursor points (`;`).
    collapse_selection,
    /// Invert anchor and head of all selections (`A-;`).
    flip_selections,
    /// Ensure all selections are oriented forward (`A-:`).
    ensure_selection_forward,
    /// Drop all secondary selections, keeping only primary (`,`).
    keep_primary_selection,
    /// Remove primary selection, promoting next surviving (`A-,`).
    remove_primary_selection,
    /// Copy selection on next line (`C`).
    copy_selection_on_next_line,
    /// Copy selection on previous line (`A-C`).
    copy_selection_on_prev_line,
    /// Cycle primary selection forward (`)`).
    rotate_selections_forward,
    /// Cycle primary selection backward (`(`).
    rotate_selections_backward,
    /// Rotate selected text contents forward across selections (`A-)`).
    rotate_selection_contents_forward,
    /// Rotate selected text contents backward across selections (`A-(`).
    rotate_selection_contents_backward,
    /// Shrink selection to line bounds (`A-x`).
    shrink_to_line_bounds,
    /// Join selected lines with space (`J`).
    join_selections,
    /// Join selected lines without space (`A-J`).
    join_selections_space,
    /// Keep selections matching regex (`K`).
    keep_matching_selections,
    /// Remove selections matching regex (`A-K`).
    remove_matching_selections,

    // --- Search ---
    /// Start forward search (`/`).
    search,
    /// Start backward search (`?`).
    rsearch,
    /// Jump to next search match (`n`).
    search_next,
    /// Jump to previous search match (`N`).
    search_prev,

    // --- LSP integration ---
    /// Jump to definition of symbol under cursor (`gd`).
    goto_definition,
    /// Jump to references of symbol under cursor (`gr`).
    goto_reference,
    /// Show hover documentation for symbol under cursor (`K`).
    hover,
    /// Jump to next diagnostic (`]d`).
    goto_next_diag,
    /// Jump to previous diagnostic (`[d`).
    goto_prev_diag,
    /// Jump to next diff hunk (`]g` or `]h`).
    goto_next_hunk,
    /// Jump to previous diff hunk (`[g` or `[h`).
    goto_prev_hunk,
    /// Open file picker (`space f`).
    file_picker,
    /// Open buffer picker (`space b`).
    buffer_picker,
    /// Open diagnostics picker (`space d`).
    diagnostics_picker,
    /// Open workspace global search picker (`space /`).
    global_search,

    // --- File operations ---
    /// Save current buffer (`:w`).
    write,
    /// Quit editor (`:q`).
    quit,
    /// Force quit without saving (`:q!`).
    force_quit,

    // --- Insert mode primitives ---
    /// Insert newline character.
    insert_newline,
    /// Insert indentation tab or spaces.
    insert_tab,
    /// Backspace delete one character backward.
    delete_char_backward,
    /// Delete word backward (`C-w`).
    delete_word_backward,

    // --- View alignment & scrolling ---
    /// Align current line to center of view (`zz` or `zc`).
    align_view_center,
    /// Align current line to top of view (`zt`).
    align_view_top,
    /// Align current line to bottom of view (`zb`).
    align_view_bottom,
    /// Scroll viewport down (`zj` or `C-d`).
    scroll_down,
    /// Scroll viewport up (`zk` or `C-u`).
    scroll_up,

    // --- Match & Surround ---
    /// Jump between matching brackets (`mm`).
    match_brackets,
    /// Select around textobject (`ma`).
    select_textobject_around,
    /// Select inside textobject (`mi`).
    select_textobject_inside,
    /// Surround selections with delimiters (`ms`).
    surround_add,
    /// Replace enclosing delimiters around selections (`mr`).
    surround_replace,
    /// Delete enclosing delimiters around selections (`md`).
    surround_delete,

    // --- Structural syntax navigation ---
    /// Expand selection to parent syntax node (`A-o`).
    expand_selection,
    /// Shrink selection to child syntax node (`A-i`).
    shrink_selection,
    /// Select previous sibling syntax node (`A-p`).
    select_prev_sibling,
    /// Select next sibling syntax node (`A-n`).
    select_next_sibling,

    // --- Window / View management ---
    /// Split view vertically (`C-w v`).
    vsplit,
    /// Split view horizontally (`C-w s`).
    hsplit,
    /// Rotate view focus (`C-w w`).
    rotate_view,
    /// Close active view window (`C-w q`).
    wclose,
    /// Close other view windows (`C-w o`).
    wonly,
    /// Jump to view on left (`C-w h`).
    jump_view_left,
    /// Jump to view below (`C-w j`).
    jump_view_down,
    /// Jump to view above (`C-w k`).
    jump_view_up,
    /// Jump to view on right (`C-w l`).
    jump_view_right,

    // --- No operation ---
    /// No operation.
    no_op,
}
