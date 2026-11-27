//! Key event, key code, modifier flags, and editor mode types.

use std::fmt;

use serde::{Deserialize, Serialize};

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
    #[inline]
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        super::parser::parse_key_event(s)
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
