//! Modal keymaps, key event parsing, and editor action definitions.
//!
//! Replicates Helix modal editing key semantics (Normal, Insert, Select modes,
//! `g` goto sub-layer, `[` and `]` brackets) using Ned's native types.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use toml::Value;

/// Editor operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum mode {
    /// Normal navigation and editing mode.
    normal,
    /// Text insertion mode.
    insert,
    /// Selection extension mode.
    select,
}

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
    /// Convert selection to uppercase (`A-` `).
    switch_to_uppercase,
    /// Indent selected lines (`>`).
    indent,
    /// Unindent selected lines (`<`).
    unindent,

    // --- Selection operations ---
    /// Select entire document (`%`).
    select_all,
    /// Collapse selections to single-character cursor points (`;`).
    collapse_selection,
    /// Invert anchor and head of all selections (`A-;`).
    flip_selections,
    /// Drop all secondary selections, keeping only primary (`,`).
    keep_primary_selection,

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
    /// Open file picker (`space f`).
    file_picker,
    /// Open buffer picker (`space b`).
    buffer_picker,

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

    // --- No operation ---
    /// No operation.
    no_op,
}

/// Represents the key code component of a keystroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum key_code {
    /// Printable character.
    char(char),
    /// Function key (1–12).
    f(u8),
    /// Backspace key.
    backspace,
    /// Enter / Return key.
    enter,
    /// Left arrow.
    left,
    /// Right arrow.
    right,
    /// Up arrow.
    up,
    /// Down arrow.
    down,
    /// Home key.
    home,
    /// End key.
    end,
    /// Page Up.
    page_up,
    /// Page Down.
    page_down,
    /// Tab key.
    tab,
    /// Delete key.
    delete,
    /// Escape key.
    escape,
}

/// Key modifier flags (Shift, Ctrl, Alt).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct key_modifiers {
    /// Shift key pressed.
    pub shift: bool,
    /// Control key pressed.
    pub ctrl: bool,
    /// Alt / Option key pressed.
    pub alt: bool,
}

/// A parsed keyboard event consisting of a key code and modifier flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct key_event {
    /// Key code pressed.
    pub code: key_code,
    /// Active modifiers.
    pub modifiers: key_modifiers,
}

impl key_event {
    /// Creates a key event from code and modifiers.
    #[must_use]
    pub const fn new(code: key_code, modifiers: key_modifiers) -> Self {
        Self { code, modifiers }
    }

    /// Creates an unmodified character key event.
    #[must_use]
    pub const fn char(ch: char) -> Self {
        Self {
            code: key_code::char(ch),
            modifiers: key_modifiers {
                shift: false,
                ctrl: false,
                alt: false,
            },
        }
    }

    /// Parses a Helix-formatted key string (e.g. `"h"`, `"A-;"`, `"C-s"`, `"space"`, `"ret"`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let mut modifiers = key_modifiers::default();
        let mut rem = s;

        // Extract modifier prefixes
        loop {
            if let Some(rest) = rem.strip_prefix("C-") {
                modifiers.ctrl = true;
                rem = rest;
            } else if let Some(rest) = rem.strip_prefix("A-") {
                modifiers.alt = true;
                rem = rest;
            } else if let Some(rest) = rem.strip_prefix("S-") {
                modifiers.shift = true;
                rem = rest;
            } else {
                break;
            }
        }

        let code = match rem {
            "space" => key_code::char(' '),
            "ret" | "return" | "enter" => key_code::enter,
            "esc" | "escape" => key_code::escape,
            "tab" => key_code::tab,
            "backspace" => key_code::backspace,
            "del" | "delete" => key_code::delete,
            "left" => key_code::left,
            "right" => key_code::right,
            "up" => key_code::up,
            "down" => key_code::down,
            "home" => key_code::home,
            "end" => key_code::end,
            "pageup" => key_code::page_up,
            "pagedown" => key_code::page_down,
            "minus" => key_code::char('-'),
            s if s.starts_with('F') || s.starts_with('f') => {
                if let Ok(num) = s[1..].parse::<u8>() {
                    key_code::f(num)
                } else {
                    key_code::char(s.chars().next()?)
                }
            }
            s if s.chars().count() == 1 => key_code::char(s.chars().next()?),
            _ => return None,
        };

        Some(Self { code, modifiers })
    }
}

impl fmt::Display for key_event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.ctrl {
            write!(f, "C-")?;
        }
        if self.modifiers.alt {
            write!(f, "A-")?;
        }
        if self.modifiers.shift {
            write!(f, "S-")?;
        }
        match self.code {
            key_code::char(' ') => write!(f, "space"),
            key_code::char(c) => write!(f, "{c}"),
            key_code::f(n) => write!(f, "F{n}"),
            key_code::backspace => write!(f, "backspace"),
            key_code::enter => write!(f, "ret"),
            key_code::left => write!(f, "left"),
            key_code::right => write!(f, "right"),
            key_code::up => write!(f, "up"),
            key_code::down => write!(f, "down"),
            key_code::home => write!(f, "home"),
            key_code::end => write!(f, "end"),
            key_code::page_up => write!(f, "pageup"),
            key_code::page_down => write!(f, "pagedown"),
            key_code::tab => write!(f, "tab"),
            key_code::delete => write!(f, "del"),
            key_code::escape => write!(f, "esc"),
        }
    }
}

/// A node or terminal action in the prefix key dispatch trie.
#[derive(Debug, Clone, PartialEq)]
pub enum key_trie {
    /// Terminal action to execute.
    action(action),
    /// Sequence of actions to execute sequentially.
    sequence(Vec<action>),
    /// Sub-trie containing nested prefix keys.
    node(key_trie_node),
}

/// Interior node in the keymap prefix tree.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct key_trie_node {
    /// Human-readable label for which-key hints.
    pub label: String,
    /// Mapping of key event to sub-trie or action.
    pub map: HashMap<key_event, key_trie>,
}

impl key_trie_node {
    /// Creates a new trie node.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            map: HashMap::new(),
        }
    }

    /// Inserts a key-action binding.
    pub fn insert(&mut self, key_str: &str, act: action) {
        if let Some(ev) = key_event::parse(key_str) {
            self.map.insert(ev, key_trie::action(act));
        }
    }

    /// Inserts a nested sub-node (prefix key).
    pub fn insert_node(&mut self, key_str: &str, sub_node: Self) {
        if let Some(ev) = key_event::parse(key_str) {
            self.map.insert(ev, key_trie::node(sub_node));
        }
    }

    /// Looks up a key event in this node.
    #[must_use]
    pub fn get(&self, key: &key_event) -> Option<&key_trie> {
        self.map.get(key)
    }

    /// Deep merges another trie node into this one, with `other` taking priority.
    pub fn merge(&mut self, other: Self) {
        for (key, other_trie) in other.map {
            match (self.map.get_mut(&key), other_trie) {
                (Some(key_trie::node(curr_node)), key_trie::node(new_node)) => {
                    curr_node.merge(new_node);
                }
                (_, new_trie) => {
                    self.map.insert(key, new_trie);
                }
            }
        }
    }
}

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

/// Parses user keymap configuration from TOML into [`key_trie_node`] hierarchies.
#[must_use]
pub fn parse_keymaps_from_toml(val: &Value) -> HashMap<mode, key_trie_node> {
    let mut res = HashMap::new();
    let table = match val.as_table() {
        Some(t) => t,
        None => return res,
    };

    let parse_mode_table = |sub_val: &Value, label: &str| -> key_trie_node {
        let mut node = key_trie_node::new(label);
        if let Some(sub_table) = sub_val.as_table() {
            for (key_str, target) in sub_table {
                match target {
                    Value::String(_) => {
                        if let Ok(act) = target.clone().try_into::<action>() {
                            node.insert(key_str, act);
                        }
                    }
                    Value::Table(_) => {
                        let child_node = parse_keymap_node(target, key_str);
                        node.insert_node(key_str, child_node);
                    }
                    _ => {}
                }
            }
        }
        node
    };

    if let Some(normal_tbl) = table.get("normal") {
        res.insert(mode::normal, parse_mode_table(normal_tbl, "Normal mode"));
    }
    if let Some(insert_tbl) = table.get("insert") {
        res.insert(mode::insert, parse_mode_table(insert_tbl, "Insert mode"));
    }
    if let Some(select_tbl) = table.get("select") {
        res.insert(mode::select, parse_mode_table(select_tbl, "Select mode"));
    }

    res
}

fn parse_keymap_node(val: &Value, label: &str) -> key_trie_node {
    let mut node = key_trie_node::new(label);
    if let Some(tbl) = val.as_table() {
        for (key_str, target) in tbl {
            match target {
                Value::String(_) => {
                    if let Ok(act) = target.clone().try_into::<action>() {
                        node.insert(key_str, act);
                    }
                }
                Value::Table(_) => {
                    let child = parse_keymap_node(target, key_str);
                    node.insert_node(key_str, child);
                }
                _ => {}
            }
        }
    }
    node
}
