//! Mode dispatching and keyboard event routing for the editor.

use ned_config::{key_code, key_event, key_trie, mode};

use crate::editors::editor;

/// Handles an incoming keyboard event by dispatching against the active mode keymap or prefix trie.
///
/// Returns `true` if the key event was handled and may require screen redraw.
pub fn handle_key(ed: &mut editor, ev: key_event) -> bool {
    ed.hover_popup = None;
    if ed.mode == mode::insert {
        return handle_insert_key(ed, ev);
    }

    handle_modal_key(ed, ev)
}

/// Dispatches a key event in Insert mode.
fn handle_insert_key(ed: &mut editor, ev: key_event) -> bool {
    // Check insert mode keybindings first (e.g. Esc -> normal_mode, Backspace)
    if let Some(trie) = ed.config.keys.get(&mode::insert) {
        if let Some(entry) = trie.get(&ev) {
            match entry {
                key_trie::action(act) => {
                    let a = *act;
                    ed.dispatch(a);
                    return true;
                }
                key_trie::sequence(acts) => {
                    let actions = acts.clone();
                    for act in actions {
                        ed.dispatch(act);
                    }
                    return true;
                }
                key_trie::node(_) => {}
            }
        }
    }

    // Insert character literal if key is a character without Ctrl/Alt modifiers
    if !ev.modifiers.ctrl && !ev.modifiers.alt {
        if let key_code::char(ch) = ev.code {
            ed.insert_char(ch);
            return true;
        }
    }

    false
}

/// Dispatches a key event in Normal or Select mode.
fn handle_modal_key(ed: &mut editor, ev: key_event) -> bool {
    // 1. Active prefix sub-trie (e.g. `g`, `z`, `Space`, `[`, `]`)
    if let Some(prefix_node) = ed.key_prefix.take() {
        if let Some(entry) = prefix_node.get(&ev) {
            match entry {
                key_trie::action(act) => {
                    let a = *act;
                    ed.dispatch(a);
                }
                key_trie::sequence(acts) => {
                    let actions = acts.clone();
                    for act in actions {
                        ed.dispatch(act);
                    }
                }
                key_trie::node(sub_node) => {
                    ed.key_prefix = Some(sub_node.clone());
                }
            }
            return true;
        }
        // Key not in prefix trie: prefix cancelled
        return true;
    }

    // 2. Top-level mode keymap lookup
    if let Some(trie) = ed.config.keys.get(&ed.mode) {
        if let Some(entry) = trie.get(&ev) {
            match entry {
                key_trie::action(act) => {
                    let a = *act;
                    ed.dispatch(a);
                }
                key_trie::sequence(acts) => {
                    let actions = acts.clone();
                    for act in actions {
                        ed.dispatch(act);
                    }
                }
                key_trie::node(sub_node) => {
                    ed.key_prefix = Some(sub_node.clone());
                }
            }
            return true;
        }
    }

    // 3. Register prefix shortcut (`"`)
    if ev.code == key_code::char('"') && !ev.modifiers.ctrl && !ev.modifiers.alt {
        return true;
    }

    false
}
