//! Parsing logic for Helix-style key specifications and TOML keymap tables.

use std::collections::HashMap;

use toml::Value;

use super::{action, key_code, key_event, key_modifiers, key_trie_node, mode};

/// Parses a Helix-formatted key string (e.g. `"h"`, `"A-;"`, `"C-s"`, `"space"`, `"ret"`).
#[must_use]
pub fn parse_key_event(s: &str) -> Option<key_event> {
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

    Some(key_event { code, modifiers })
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
