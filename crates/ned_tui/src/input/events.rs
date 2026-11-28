//! The input event model

use crate::prelude::Point;
use std::collections::HashSet;

/// The `AppEvent` is an extension point in termit so that
/// you can include your application specific event stream
/// in the input processing.
pub trait AppEvent: std::fmt::Debug + PartialEq + Unpin + 'static {}
impl<T> AppEvent for T where T: std::fmt::Debug + PartialEq + Unpin + 'static {}

/// The main input entity extensible with application specific [`AppEvent`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<A: AppEvent = ()> {
    /// Keyboard key press or release.
    Key(KeyEvent),
    /// Terminal focus gained or lost.
    Focus(bool),
    /// Cursor position report.
    CursorPosition(Point),
    /// Terminal resize event.
    Resize(Point),
    /// Mouse movement or click event.
    Mouse(MouseEvent),
    /// Bracketed paste string content.
    Paste(String),
    /// Unrecognized raw escape sequence bytes.
    Unknown(Vec<u8>),
    /// Refresh with n beats passed.
    Refresh(usize),
    /// Application specific event.
    App(A),
    /// The terminal input has been closed.
    InputClosed,
}

/// Key code representing a keyboard key.
#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    /// Escape key.
    Esc,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Function key F1-F12.
    F(u8),
    /// Shift+Tab (back tab).
    BackTab,
    /// Enter/Return key.
    Enter,
    /// Tab key.
    Tab,
    /// Backspace key.
    Backspace,
    /// Printable character.
    Char(char),
    /// Insert key.
    Insert,
    /// Delete key.
    Delete,
    /// Page Up key.
    PageUp,
    /// Page Down key.
    PageDown,
    /// Keypad center/5 key.
    KeypadBegin,
    /// Caps lock toggle.
    CapsLock,
    /// Scroll lock toggle.
    ScrollLock,
    /// Num lock toggle.
    NumLock,
    /// Print screen key.
    PrintScreen,
    /// Pause/Break key.
    Pause,
    /// Context menu key.
    Menu,
    /// Multimedia key.
    Media(MediaKeyCode),
    /// Standalone modifier key press.
    Modifier(ModifierKeyCode),
}

/// Media key codes for keyboard multimedia keys.
#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKeyCode {
    /// Play media.
    Play,
    /// Pause media.
    Pause,
    /// Toggle play/pause.
    PlayPause,
    /// Reverse media.
    Reverse,
    /// Stop media.
    Stop,
    /// Fast forward media.
    FastForward,
    /// Rewind media.
    Rewind,
    /// Next track.
    TrackNext,
    /// Previous track.
    TrackPrevious,
    /// Record media.
    Record,
    /// Lower audio volume.
    LowerVolume,
    /// Raise audio volume.
    RaiseVolume,
    /// Mute audio volume.
    MuteVolume,
}

/// Modifier key codes.
#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierKeyCode {
    /// Left Shift key.
    LeftShift,
    /// Left Control key.
    LeftControl,
    /// Left Alt key.
    LeftAlt,
    /// Left Super (Windows/Command) key.
    LeftSuper,
    /// Left Hyper key.
    LeftHyper,
    /// Left Meta key.
    LeftMeta,
    /// Right Shift key.
    RightShift,
    /// Right Control key.
    RightControl,
    /// Right Alt key.
    RightAlt,
    /// Right Super key.
    RightSuper,
    /// Right Hyper key.
    RightHyper,
    /// Right Meta key.
    RightMeta,
    /// ISO level 3 shift.
    IsoLevel3Shift,
    /// ISO level 5 shift.
    IsoLevel5Shift,
}

/// Keyboard event structure containing keycode and modifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    /// The key code that was pressed.
    pub keycode: KeyCode,
    /// The event kind (press, release, repeat).
    pub kind: KeyEventKind,
    /// Active modifiers (ctrl, alt, shift).
    pub modifiers: KeyModifiers,
    /// Keyboard lock state.
    pub state: KeyEventStates,
}

impl KeyEvent {
    pub(crate) fn new(
        keycode: KeyCode,
        modifiers: impl IntoIterator<Item = KeyModifier>,
    ) -> KeyEvent {
        KeyEvent::new_with_kind(keycode, modifiers, KeyEventKind::Press)
    }

    pub(crate) fn new_with_kind(
        keycode: KeyCode,
        modifiers: impl IntoIterator<Item = KeyModifier>,
        kind: KeyEventKind,
    ) -> KeyEvent {
        KeyEvent::new_with_kind_and_state(keycode, modifiers, kind, None)
    }

    pub(crate) fn new_with_kind_and_state(
        keycode: KeyCode,
        modifiers: impl IntoIterator<Item = KeyModifier>,
        kind: KeyEventKind,
        state: impl IntoIterator<Item = KeyEventState>,
    ) -> KeyEvent {
        KeyEvent {
            keycode,
            modifiers: modifiers.into_iter().collect(),
            kind,
            state: state.into_iter().collect(),
        }
    }
}

impl From<KeyCode> for KeyEvent {
    fn from(keycode: KeyCode) -> Self {
        let modifiers = match keycode {
            KeyCode::Char(c) if c.is_uppercase() => Some(KeyModifier::Shift),
            _ => None,
        };
        KeyEvent::new(keycode, modifiers)
    }
}

/// Set of active key modifiers.
pub type KeyModifiers = HashSet<KeyModifier>;

/// Modifier key representation.
#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyModifier {
    /// Alt key.
    Alt,
    /// Control key.
    Control,
    /// Shift key.
    Shift,
    /// Super key.
    Super,
    /// Hyper key.
    Hyper,
    /// Meta key.
    Meta,
}

/// Kind of key event (press, release, repeat).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum KeyEventKind {
    /// Key was pressed down.
    Press,
    /// Key was released.
    Release,
    /// Key is repeating from being held.
    Repeat,
}

/// Set of key event lock states.
pub type KeyEventStates = HashSet<KeyEventState>;

/// Lock state during key event.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum KeyEventState {
    /// Caps lock active.
    CapsLock,
    /// Num lock active.
    NumLock,
    /// Keypad origin.
    Keypad,
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum MouseButton {
    /// Primary/left button.
    Left,
    /// Middle/wheel button.
    Middle,
    /// Secondary/right button.
    Right,
}

/// Terminal mouse event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseEvent {
    /// Column position (0-indexed).
    pub column: u16,
    /// Row position (0-indexed).
    pub row: u16,
    /// Action kind (click, move, drag, scroll).
    pub kind: MouseEventKind,
    /// Modifiers held during mouse event.
    pub modifiers: KeyModifiers,
}

/// Kind of mouse event action.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum MouseEventKind {
    /// Mouse pointer moved.
    Moved,
    /// Button released.
    Up(MouseButton),
    /// Mouse dragged with button held.
    Drag(MouseButton),
    /// Button pressed down.
    Down(MouseButton),
    /// Wheel scrolled down.
    ScrollDown,
    /// Wheel scrolled up.
    ScrollUp,
}
