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
#[allow(clippy::too_many_lines)]
fn handle_modal_key(ed: &mut editor, ev: key_event) -> bool {
    // 1. Escape clears any pending prefix, count, register selection, and status message
    if ev.code == key_code::escape {
        let had_prefix = ed.key_prefix.is_some()
            || ed.count.is_some()
            || ed.selected_register.is_some()
            || ed.pending_replace
            || ed.pending_textobject.is_some()
            || ed.pending_surround.is_some();
        ed.key_prefix = None;
        ed.count = None;
        ed.selected_register = None;
        ed.pending_replace = false;
        ed.pending_textobject = None;
        ed.pending_surround = None;
        if had_prefix {
            ed.status_message = None;
            return true;
        }
        if ed.mode == mode::select {
            ed.mode = mode::normal;
            return true;
        }
        return true;
    }

    // 2. Pending textobject selection capture (`ma` / `mi`)
    if let Some(to_mode) = ed.pending_textobject.take() {
        ed.status_message = None;
        let count = ed.count.take().unwrap_or(1);
        if let key_code::char(ch) = ev.code {
            if !ev.modifiers.ctrl && !ev.modifiers.alt {
                crate::editors::edit_ops::apply_textobject(ed, to_mode, ch, count);
                return true;
            }
        }
        return true;
    }

    // 3. Pending surround operations (`ms`, `md`, `mr`)
    if let Some(op) = ed.pending_surround.take() {
        let count = ed.count.take().unwrap_or(1);
        match op {
            crate::surround::surround_op::add => {
                ed.status_message = None;
                let ch = match ev.code {
                    key_code::char(c) if !ev.modifiers.ctrl && !ev.modifiers.alt => Some(c),
                    key_code::enter => Some('\n'),
                    _ => None,
                };
                if let Some(c) = ch {
                    if let Err(msg) = crate::surround::surround_add(ed.active_doc_mut(), c) {
                        ed.status_message = Some(msg.to_string());
                    }
                    if ed.mode == mode::select {
                        ed.mode = mode::normal;
                    }
                }
                return true;
            }
            crate::surround::surround_op::delete => {
                ed.status_message = None;
                if let key_code::char(c) = ev.code {
                    if !ev.modifiers.ctrl && !ev.modifiers.alt {
                        if let Err(msg) =
                            crate::surround::surround_delete(ed.active_doc_mut(), c, count)
                        {
                            ed.status_message = Some(msg.to_string());
                        }
                        if ed.mode == mode::select {
                            ed.mode = mode::normal;
                        }
                    }
                }
                return true;
            }
            crate::surround::surround_op::replace_first => {
                if let key_code::char(c) = ev.code {
                    if !ev.modifiers.ctrl && !ev.modifiers.alt {
                        ed.pending_surround = Some(crate::surround::surround_op::replace_second(c));
                        ed.count = Some(count);
                        ed.status_message = Some(format!("surround replace '{c}' with: "));
                        return true;
                    }
                }
                ed.status_message = None;
                return true;
            }
            crate::surround::surround_op::replace_second(old_ch) => {
                ed.status_message = None;
                if let key_code::char(new_ch) = ev.code {
                    if !ev.modifiers.ctrl && !ev.modifiers.alt {
                        if let Err(msg) = crate::surround::surround_replace(
                            ed.active_doc_mut(),
                            old_ch,
                            new_ch,
                            count,
                        ) {
                            ed.status_message = Some(msg.to_string());
                        }
                        if ed.mode == mode::select {
                            ed.mode = mode::normal;
                        }
                    }
                }
                return true;
            }
        }
    }

    // 4. Pending replace character capture (`r`)
    if ed.pending_replace {
        ed.pending_replace = false;
        ed.status_message = None;
        if let key_code::char(ch) = ev.code {
            if !ev.modifiers.ctrl && !ev.modifiers.alt {
                ed.replace_selection_with_char(ch);
                return true;
            }
        }
        return true;
    }

    // 3. Register selection char capture (when `"` was previously pressed)
    if ed.selected_register == Some('\0') {
        if let key_code::char(ch) = ev.code {
            if !ev.modifiers.ctrl && !ev.modifiers.alt {
                let is_valid = ch.is_ascii_alphanumeric()
                    || ch == '"'
                    || ch == '_'
                    || ch == '.'
                    || ch == '#'
                    || ch == '%'
                    || ch == '/'
                    || ch == ':'
                    || ch == '+'
                    || ch == '*';
                if is_valid {
                    ed.selected_register = Some(ch);
                    ed.status_message = Some(format!("register: \"{ch}"));
                    return true;
                }
            }
        }
        ed.selected_register = None;
        ed.status_message = None;
        return true;
    }

    // 3. Active prefix sub-trie (e.g. `g`, `z`, `Space`, `[`, `]`, `m`, `C-w`)
    if let Some(prefix_node) = ed.key_prefix.take() {
        if let Some(entry) = prefix_node.get(&ev) {
            match entry {
                key_trie::action(act) => {
                    let a = *act;
                    ed.status_message = None;
                    ed.dispatch(a);
                    ed.selected_register = None;
                }
                key_trie::sequence(acts) => {
                    let actions = acts.clone();
                    ed.status_message = None;
                    for act in actions {
                        ed.dispatch(act);
                    }
                    ed.selected_register = None;
                }
                key_trie::node(sub_node) => {
                    ed.key_prefix = Some(sub_node.clone());
                }
            }
            return true;
        }
        // Key not in prefix trie: prefix cancelled
        ed.count = None;
        ed.selected_register = None;
        ed.status_message = None;
        return true;
    }

    // 4. Register prefix shortcut (`"`)
    if ev.code == key_code::char('"') && !ev.modifiers.ctrl && !ev.modifiers.alt {
        ed.selected_register = Some('\0');
        ed.status_message = Some("register: \"".to_string());
        return true;
    }

    // 5. Numerical count prefix (`1`..=`9`, or `0`..=`9` when count is already started)
    if !ev.modifiers.ctrl && !ev.modifiers.alt {
        if let key_code::char(ch) = ev.code {
            if ch.is_ascii_digit() && (ch != '0' || ed.count.is_some()) {
                let digit = ch.to_digit(10).unwrap() as usize;
                let current = ed.count.unwrap_or(0);
                let new_count = current.saturating_mul(10).saturating_add(digit).min(99999);
                ed.count = Some(new_count);
                ed.status_message = Some(format!("count: {new_count}"));
                return true;
            }
        }
    }

    // 6. Top-level mode keymap lookup
    if let Some(trie) = ed.config.keys.get(&ed.mode) {
        if let Some(entry) = trie.get(&ev) {
            match entry {
                key_trie::action(act) => {
                    let a = *act;
                    if let Some(msg) = &ed.status_message {
                        if msg.starts_with("count: ") || msg.starts_with("register: ") {
                            ed.status_message = None;
                        }
                    }
                    ed.dispatch(a);
                    ed.selected_register = None;
                }
                key_trie::sequence(acts) => {
                    let actions = acts.clone();
                    if let Some(msg) = &ed.status_message {
                        if msg.starts_with("count: ") || msg.starts_with("register: ") {
                            ed.status_message = None;
                        }
                    }
                    for act in actions {
                        ed.dispatch(act);
                    }
                    ed.selected_register = None;
                }
                key_trie::node(sub_node) => {
                    ed.key_prefix = Some(sub_node.clone());
                }
            }
            return true;
        }
    }

    // Unbound key clears count and register
    ed.count = None;
    ed.selected_register = None;
    false
}
