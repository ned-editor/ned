//! Keyboard event translation from `ned_tui` to `ned_config`.

use ned_config::{key_code, key_event, key_modifiers};
use ned_tui::input::events::{KeyCode, KeyEvent, KeyModifier};

/// Translates a `ned_tui` key event into a `ned_config` modal key event.
#[must_use]
pub fn translate_key_event(ev: &KeyEvent) -> Option<key_event> {
    let mods = &ev.modifiers;
    let ctrl = mods.contains(&KeyModifier::Control);
    let alt = mods.contains(&KeyModifier::Alt);
    let shift = mods.contains(&KeyModifier::Shift);

    let (code, effective_shift) = match ev.keycode {
        KeyCode::Esc => (key_code::escape, shift),
        KeyCode::Enter => (key_code::enter, shift),
        KeyCode::Tab => (key_code::tab, shift),
        KeyCode::BackTab => (key_code::tab, true),
        KeyCode::Backspace => (key_code::backspace, shift),
        KeyCode::Delete => (key_code::delete, shift),
        KeyCode::Left => (key_code::left, shift),
        KeyCode::Right => (key_code::right, shift),
        KeyCode::Up => (key_code::up, shift),
        KeyCode::Down => (key_code::down, shift),
        KeyCode::Home => (key_code::home, shift),
        KeyCode::End => (key_code::end, shift),
        KeyCode::PageUp => (key_code::page_up, shift),
        KeyCode::PageDown => (key_code::page_down, shift),
        KeyCode::F(n) => (key_code::f(n), shift),
        KeyCode::Char(ch) => {
            // In modal keymaps, character case is embedded directly in the char literal.
            // Shift is only kept if Ctrl or Alt is also active with an uppercase char.
            (key_code::char(ch), false)
        }
        _ => return None,
    };

    Some(key_event::new(
        code,
        key_modifiers {
            shift: effective_shift,
            ctrl,
            alt,
        },
    ))
}
