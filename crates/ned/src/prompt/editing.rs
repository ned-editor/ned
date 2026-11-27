//! Prompt line editing operations and readline-style keyboard navigation.

use ned_tui::input::events::{KeyCode, KeyEvent, KeyModifier};

use super::completion_direction;
use super::state::{prompt_action, prompt_state};

/// Handles prompt key events coordinating navigation, line editing, and completions.
pub fn handle_key(prompt: &mut prompt_state, ev: &KeyEvent) -> prompt_action {
    let ctrl = ev.modifiers.contains(&KeyModifier::Control);
    let alt = ev.modifiers.contains(&KeyModifier::Alt);
    let shift = ev.modifiers.contains(&KeyModifier::Shift);

    match ev.keycode {
        // Esc: First press dismisses completion selection; second press cancels prompt
        KeyCode::Esc => {
            if prompt.selection.is_some() {
                prompt.exit_selection();
                prompt_action::none
            } else {
                prompt_action::cancel
            }
        }

        // Enter: Submits current prompt input
        KeyCode::Enter => prompt_action::submit(prompt.input.clone()),

        // Tab / Down: Cycle completion forward
        KeyCode::Tab if !shift => {
            prompt.change_completion_selection(completion_direction::forward);
            prompt_action::none
        }
        KeyCode::BackTab | KeyCode::Tab if shift => {
            prompt.change_completion_selection(completion_direction::backward);
            prompt_action::none
        }
        KeyCode::Down => {
            prompt.change_completion_selection(completion_direction::forward);
            prompt_action::none
        }
        KeyCode::Up => {
            prompt.change_completion_selection(completion_direction::backward);
            prompt_action::none
        }

        // Readline navigation
        KeyCode::Home => {
            prompt.cursor = 0;
            prompt_action::none
        }
        KeyCode::End => {
            prompt.cursor = prompt.input.chars().count();
            prompt_action::none
        }
        KeyCode::Left => {
            if prompt.cursor > 0 {
                prompt.cursor -= 1;
            }
            prompt_action::none
        }
        KeyCode::Right => {
            if prompt.cursor < prompt.input.chars().count() {
                prompt.cursor += 1;
            }
            prompt_action::none
        }

        // Deletions
        KeyCode::Backspace => {
            if ctrl || alt {
                delete_word_backward(prompt);
                prompt_action::none
            } else if prompt.cursor > 0 {
                delete_char_backward(prompt);
                prompt_action::none
            } else {
                prompt_action::cancel
            }
        }
        KeyCode::Delete => {
            if prompt.cursor < prompt.input.chars().count() {
                prompt.input.remove(prompt.cursor);
                prompt.recalculate_completion();
            }
            prompt_action::none
        }

        // Character input & Ctrl readline shortcuts
        KeyCode::Char(ch) => {
            if ctrl {
                match ch {
                    'a' => {
                        prompt.cursor = 0;
                        prompt_action::none
                    }
                    'e' => {
                        prompt.cursor = prompt.input.chars().count();
                        prompt_action::none
                    }
                    'u' => {
                        prompt.input.drain(..prompt.cursor);
                        prompt.cursor = 0;
                        prompt.recalculate_completion();
                        prompt_action::none
                    }
                    'k' => {
                        prompt.input.truncate(prompt.cursor);
                        prompt.recalculate_completion();
                        prompt_action::none
                    }
                    'w' => {
                        delete_word_backward(prompt);
                        prompt_action::none
                    }
                    'c' => prompt_action::cancel,
                    _ => prompt_action::none,
                }
            } else {
                insert_char(prompt, ch);
                prompt_action::none
            }
        }
        _ => prompt_action::none,
    }
}

/// Inserts a character at the current cursor position and advances the cursor.
pub fn insert_char(prompt: &mut prompt_state, ch: char) {
    prompt.input.insert(prompt.cursor, ch);
    prompt.cursor += 1;
    prompt.recalculate_completion();
}

/// Deletes the character immediately before the cursor.
pub fn delete_char_backward(prompt: &mut prompt_state) {
    if prompt.cursor > 0 {
        prompt.cursor -= 1;
        prompt.input.remove(prompt.cursor);
        prompt.recalculate_completion();
    }
}

/// Deletes the word immediately before the cursor.
pub fn delete_word_backward(prompt: &mut prompt_state) {
    while prompt.cursor > 0 && prompt.input.chars().nth(prompt.cursor - 1) == Some(' ') {
        prompt.cursor -= 1;
        prompt.input.remove(prompt.cursor);
    }
    while prompt.cursor > 0 && prompt.input.chars().nth(prompt.cursor - 1) != Some(' ') {
        prompt.cursor -= 1;
        prompt.input.remove(prompt.cursor);
    }
    prompt.recalculate_completion();
}
