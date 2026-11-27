use crate::{input::events::*, prelude::point};

type Result<T> = std::result::Result<T, &'static str>;

// Event<A> parsing
//
// This code (& previous one) are kind of ugly. We have to think about this,
// because it's really not maintainable, no tests, etc.
//
// Every fn returns Result<Option<InputEvent>>
//
// Ok(None) -> wait for more bytes
// Err(_) -> failed to parse event, clear the buffer
// Ok(Some(event)) -> we have event, clear the buffer
//

pub(crate) fn parse_esc<A: AppEvent>(buffer: &[u8], raw_mode: bool) -> Result<Option<Event<A>>> {
    assert!(!buffer.is_empty());
    Ok(Some(match buffer {
        [0x1B] => Event::Key(KeyCode::Esc.into()),
        // Incomplete SS3: the final byte has not arrived yet.
        [0x1B, b'O'] => return Ok(None),
        [0x1B, b'O', param] => match param {
            b'D' => Event::Key(KeyCode::Left.into()),
            b'C' => Event::Key(KeyCode::Right.into()),
            b'A' => Event::Key(KeyCode::Up.into()),
            b'B' => Event::Key(KeyCode::Down.into()),
            b'H' => Event::Key(KeyCode::Home.into()),
            b'F' => Event::Key(KeyCode::End.into()),
            // F1-F4
            val @ b'P'..=b'S' => Event::Key(KeyCode::F(1 + val - b'P').into()),
            _ => return Err("Invalid ESC O <p>"),
        },

        [0x1B, b'[', ..] => return parse_csi(buffer, raw_mode),
        [0x1B, b'\x1B'] => Event::Key(KeyCode::Esc.into()),
        [0x1B, ..] => {
            return parse_utf8_char(&buffer[1..])?
                .map(|c| {
                    let mut key = parse_key(c, raw_mode)?;
                    if let Some(Event::Key(key)) = &mut key {
                        key.modifiers.insert(KeyModifier::Alt);
                    }
                    Ok(key)
                })
                .transpose()
                .map(|key| key.flatten());
        }
        _ => return Err("Invalid ESC <p>"),
    }))
}

pub(crate) fn parse_key<A: AppEvent>(key: char, raw_mode: bool) -> Result<Option<Event<A>>> {
    Ok(Some(Event::Key(match key {
        '\r' => KeyCode::Enter.into(),
        // Issue #371: \n = 0xA, which is also the keycode for Ctrl+J. The only reason we get
        // newlines as input is because the terminal converts \r into \n for us. When we
        // enter raw mode, we disable that, so \n no longer has any meaning - it's better to
        // use Ctrl+J. Waiting to handle it here means it gets picked up later
        '\n' if !raw_mode => KeyCode::Enter.into(),
        '\t' => KeyCode::Tab.into(),
        '\x7F' => KeyCode::Backspace.into(),
        c @ '\x01'..='\x1A' => KeyEvent::new(
            KeyCode::Char((c as u8 - 0x1 + b'a') as char),
            Some(KeyModifier::Control),
        ),
        c @ '\x1C'..='\x1F' => KeyEvent::new(
            KeyCode::Char((c as u8 - 0x1C + b'4') as char),
            Some(KeyModifier::Control),
        ),
        '\0' => KeyEvent::new(KeyCode::Char(' '), Some(KeyModifier::Control)),
        _ => KeyCode::Char(key).into(),
    })))
}

pub(crate) fn parse_csi<A: AppEvent>(buffer: &[u8], raw_mode: bool) -> Result<Option<Event<A>>> {
    assert!(buffer.starts_with(b"\x1B[")); // ESC [

    if buffer.len() == 2 {
        return Ok(None);
    }

    let input_event = match buffer[2] {
        b'[' => {
            if buffer.len() == 3 {
                return Ok(None);
            } else {
                match buffer[3] {
                    // NOTE (@imdaveho): cannot find when this occurs;
                    // having another '[' after ESC[ not a likely scenario
                    val @ b'A'..=b'E' => Event::Key(KeyCode::F(1 + val - b'A').into()),
                    _ => return Err("Invalid ESC [ [ <p>"),
                }
            }
        }
        b'D' => Event::Key(KeyCode::Left.into()),
        b'C' => Event::Key(KeyCode::Right.into()),
        b'A' => Event::Key(KeyCode::Up.into()),
        b'B' => Event::Key(KeyCode::Down.into()),
        b'H' => Event::Key(KeyCode::Home.into()),
        b'F' => Event::Key(KeyCode::End.into()),
        b'Z' => Event::Key(KeyEvent::new_with_kind(
            KeyCode::BackTab,
            Some(KeyModifier::Shift),
            KeyEventKind::Press,
        )),
        b'M' => return parse_csi_normal_mouse(buffer),
        b'<' => return parse_csi_sgr_mouse(buffer),
        b'I' => Event::Focus(true),
        b'O' => Event::Focus(false),
        b';' => return parse_csi_modifier_key_code(buffer),
        b'0'..=b'9' => {
            // Numbered escape code.
            if buffer.len() == 3 {
                return Ok(None);
            } else {
                // The final byte of a CSI sequence can be in the range 64-126, so
                // let's keep reading anything else.
                let last_byte = buffer[buffer.len() - 1];
                if !(64..=126).contains(&last_byte) {
                    return Ok(None);
                } else {
                    if buffer.starts_with(b"\x1B[200~") {
                        return parse_csi_bracketed_paste(buffer);
                    }
                    match last_byte {
                        b'M' => return parse_csi_rxvt_mouse(buffer),
                        b'~' => return parse_csi_special_key_code(buffer),
                        b'u' => return parse_csi_u_encoded_key_code(buffer, raw_mode),
                        b'R' => return parse_csi_cursor_position(buffer),
                        _ => return parse_csi_modifier_key_code(buffer),
                    }
                }
            }
        }
        _ => return Err("Invalid CSI - ESC [ <...>"),
    };

    Ok(Some(input_event))
}

pub(crate) fn next_parsed<T>(iter: &mut dyn Iterator<Item = &str>) -> Result<T>
where
    T: std::str::FromStr,
{
    iter.next()
        .ok_or("missing item in iter")?
        .parse::<T>()
        .map_err(|_| "error parsing iter")
}

fn modifier_and_kind_parsed(iter: &mut dyn Iterator<Item = &str>) -> Result<(u8, u8)> {
    let mut sub_split = iter.next().ok_or("could not split modifier")?.split(':');

    let modifier_mask = next_parsed::<u8>(&mut sub_split)?;

    if let Ok(kind_code) = next_parsed::<u8>(&mut sub_split) {
        Ok((modifier_mask, kind_code))
    } else {
        Ok((modifier_mask, 1))
    }
}

pub(crate) fn parse_csi_cursor_position<A: AppEvent>(buffer: &[u8]) -> Result<Option<Event<A>>> {
    // ESC [ Cy ; Cx R
    //   Cy - cursor row number (starting from 1)
    //   Cx - cursor column number (starting from 1)
    assert!(buffer.starts_with(b"\x1B[")); // ESC [
    assert!(buffer.ends_with(b"R"));

    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| "could not parse utf-8 fpr CSI cursor")?;

    let mut split = s.split(';');

    let y = next_parsed::<u16>(&mut split)? - 1;
    let x = next_parsed::<u16>(&mut split)? - 1;

    Ok(Some(Event::CursorPosition(point(x, y))))
}

fn parse_modifiers(mask: u8) -> KeyModifiers {
    let modifier_mask = mask.saturating_sub(1);
    let mut modifiers = KeyModifiers::default();
    if modifier_mask & 1 != 0 {
        modifiers.insert(KeyModifier::Shift);
    }
    if modifier_mask & 2 != 0 {
        modifiers.insert(KeyModifier::Alt);
    }
    if modifier_mask & 4 != 0 {
        modifiers.insert(KeyModifier::Control);
    }
    if modifier_mask & 8 != 0 {
        modifiers.insert(KeyModifier::Super);
    }
    if modifier_mask & 16 != 0 {
        modifiers.insert(KeyModifier::Hyper);
    }
    if modifier_mask & 32 != 0 {
        modifiers.insert(KeyModifier::Meta);
    }
    modifiers
}

fn parse_modifiers_to_state(mask: u8) -> KeyEventStates {
    let modifier_mask = mask.saturating_sub(1);
    let mut state = KeyEventStates::default();
    if modifier_mask & 64 != 0 {
        state.insert(KeyEventState::CapsLock);
    }
    if modifier_mask & 128 != 0 {
        state.insert(KeyEventState::NumLock);
    }
    state
}

fn parse_key_event_kind(kind: u8) -> KeyEventKind {
    match kind {
        1 => KeyEventKind::Press,
        2 => KeyEventKind::Repeat,
        3 => KeyEventKind::Release,
        _ => KeyEventKind::Press,
    }
}

pub(crate) fn parse_csi_modifier_key_code<A: AppEvent>(buffer: &[u8]) -> Result<Option<Event<A>>> {
    assert!(buffer.starts_with(b"\x1B[")); // ESC [
                                           //
    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| "Could not parse utf-8 for CSI modifier code")?;
    let mut split = s.split(';');

    split.next();

    let (modifiers, kind) = if let Ok((modifier_mask, kind_code)) =
        modifier_and_kind_parsed(&mut split)
    {
        (
            parse_modifiers(modifier_mask),
            parse_key_event_kind(kind_code),
        )
    } else if buffer.len() > 3 {
        (
            parse_modifiers(
                (buffer[buffer.len() - 2] as char)
                    .to_digit(10)
                    .ok_or("Could not parse digits for CSI modifier code")? as u8,
            ),
            KeyEventKind::Press,
        )
    } else {
        (KeyModifiers::default(), KeyEventKind::Press)
    };
    let key = buffer[buffer.len() - 1];

    let keycode = match key {
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'C' => KeyCode::Right,
        b'D' => KeyCode::Left,
        b'F' => KeyCode::End,
        b'H' => KeyCode::Home,
        b'P' => KeyCode::F(1),
        b'Q' => KeyCode::F(2),
        b'R' => KeyCode::F(3),
        b'S' => KeyCode::F(4),
        _ => return Err("Invalid CSI modifier code"),
    };

    let input_event = Event::Key(KeyEvent::new_with_kind(keycode, modifiers, kind));

    Ok(Some(input_event))
}

fn translate_functional_key_code(codepoint: u32) -> Option<(KeyCode, KeyEventStates)> {
    if let Some(keycode) = match codepoint {
        57399 => Some(KeyCode::Char('0')),
        57400 => Some(KeyCode::Char('1')),
        57401 => Some(KeyCode::Char('2')),
        57402 => Some(KeyCode::Char('3')),
        57403 => Some(KeyCode::Char('4')),
        57404 => Some(KeyCode::Char('5')),
        57405 => Some(KeyCode::Char('6')),
        57406 => Some(KeyCode::Char('7')),
        57407 => Some(KeyCode::Char('8')),
        57408 => Some(KeyCode::Char('9')),
        57409 => Some(KeyCode::Char('.')),
        57410 => Some(KeyCode::Char('/')),
        57411 => Some(KeyCode::Char('*')),
        57412 => Some(KeyCode::Char('-')),
        57413 => Some(KeyCode::Char('+')),
        57414 => Some(KeyCode::Enter),
        57415 => Some(KeyCode::Char('=')),
        57416 => Some(KeyCode::Char(',')),
        57417 => Some(KeyCode::Left),
        57418 => Some(KeyCode::Right),
        57419 => Some(KeyCode::Up),
        57420 => Some(KeyCode::Down),
        57421 => Some(KeyCode::PageUp),
        57422 => Some(KeyCode::PageDown),
        57423 => Some(KeyCode::Home),
        57424 => Some(KeyCode::End),
        57425 => Some(KeyCode::Insert),
        57426 => Some(KeyCode::Delete),
        57427 => Some(KeyCode::KeypadBegin),
        _ => None,
    } {
        return Some((keycode, Some(KeyEventState::Keypad).into_iter().collect()));
    }

    if let Some(keycode) = match codepoint {
        57358 => Some(KeyCode::CapsLock),
        57359 => Some(KeyCode::ScrollLock),
        57360 => Some(KeyCode::NumLock),
        57361 => Some(KeyCode::PrintScreen),
        57362 => Some(KeyCode::Pause),
        57363 => Some(KeyCode::Menu),
        57376 => Some(KeyCode::F(13)),
        57377 => Some(KeyCode::F(14)),
        57378 => Some(KeyCode::F(15)),
        57379 => Some(KeyCode::F(16)),
        57380 => Some(KeyCode::F(17)),
        57381 => Some(KeyCode::F(18)),
        57382 => Some(KeyCode::F(19)),
        57383 => Some(KeyCode::F(20)),
        57384 => Some(KeyCode::F(21)),
        57385 => Some(KeyCode::F(22)),
        57386 => Some(KeyCode::F(23)),
        57387 => Some(KeyCode::F(24)),
        57388 => Some(KeyCode::F(25)),
        57389 => Some(KeyCode::F(26)),
        57390 => Some(KeyCode::F(27)),
        57391 => Some(KeyCode::F(28)),
        57392 => Some(KeyCode::F(29)),
        57393 => Some(KeyCode::F(30)),
        57394 => Some(KeyCode::F(31)),
        57395 => Some(KeyCode::F(32)),
        57396 => Some(KeyCode::F(33)),
        57397 => Some(KeyCode::F(34)),
        57398 => Some(KeyCode::F(35)),
        57428 => Some(KeyCode::Media(MediaKeyCode::Play)),
        57429 => Some(KeyCode::Media(MediaKeyCode::Pause)),
        57430 => Some(KeyCode::Media(MediaKeyCode::PlayPause)),
        57431 => Some(KeyCode::Media(MediaKeyCode::Reverse)),
        57432 => Some(KeyCode::Media(MediaKeyCode::Stop)),
        57433 => Some(KeyCode::Media(MediaKeyCode::FastForward)),
        57434 => Some(KeyCode::Media(MediaKeyCode::Rewind)),
        57435 => Some(KeyCode::Media(MediaKeyCode::TrackNext)),
        57436 => Some(KeyCode::Media(MediaKeyCode::TrackPrevious)),
        57437 => Some(KeyCode::Media(MediaKeyCode::Record)),
        57438 => Some(KeyCode::Media(MediaKeyCode::LowerVolume)),
        57439 => Some(KeyCode::Media(MediaKeyCode::RaiseVolume)),
        57440 => Some(KeyCode::Media(MediaKeyCode::MuteVolume)),
        57441 => Some(KeyCode::Modifier(ModifierKeyCode::LeftShift)),
        57442 => Some(KeyCode::Modifier(ModifierKeyCode::LeftControl)),
        57443 => Some(KeyCode::Modifier(ModifierKeyCode::LeftAlt)),
        57444 => Some(KeyCode::Modifier(ModifierKeyCode::LeftSuper)),
        57445 => Some(KeyCode::Modifier(ModifierKeyCode::LeftHyper)),
        57446 => Some(KeyCode::Modifier(ModifierKeyCode::LeftMeta)),
        57447 => Some(KeyCode::Modifier(ModifierKeyCode::RightShift)),
        57448 => Some(KeyCode::Modifier(ModifierKeyCode::RightControl)),
        57449 => Some(KeyCode::Modifier(ModifierKeyCode::RightAlt)),
        57450 => Some(KeyCode::Modifier(ModifierKeyCode::RightSuper)),
        57451 => Some(KeyCode::Modifier(ModifierKeyCode::RightHyper)),
        57452 => Some(KeyCode::Modifier(ModifierKeyCode::RightMeta)),
        57453 => Some(KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift)),
        57454 => Some(KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift)),
        _ => None,
    } {
        return Some((keycode, KeyEventStates::default()));
    }

    None
}

pub(crate) fn parse_csi_u_encoded_key_code<A: AppEvent>(
    buffer: &[u8],
    raw_mode: bool,
) -> Result<Option<Event<A>>> {
    assert!(buffer.starts_with(b"\x1B[")); // ESC [
    assert!(buffer.ends_with(b"u"));

    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| "Could not parse UTF-8 for CSI encoded key code")?;
    let mut split = s.split(';');

    // This CSI sequence a tuple of semicolon-separated numbers.
    // CSI [codepoint];[modifiers] u
    // codepoint: ASCII Dec value
    let codepoint = next_parsed::<u32>(&mut split)?;

    let (mut modifiers, kind, mut state) =
        if let Ok((modifier_mask, kind_code)) = modifier_and_kind_parsed(&mut split) {
            (
                parse_modifiers(modifier_mask),
                parse_key_event_kind(kind_code),
                parse_modifiers_to_state(modifier_mask),
            )
        } else {
            (
                KeyModifiers::default(),
                KeyEventKind::Press,
                KeyEventStates::default(),
            )
        };

    let (keycode, state_from_keycode) = {
        if let Some((special_key_code, state)) = translate_functional_key_code(codepoint) {
            (special_key_code, state)
        } else if let Some(mut c) = char::from_u32(codepoint) {
            (
                match c {
                    '\x1B' => KeyCode::Esc,
                    '\r' => KeyCode::Enter,
                    // Issue #371: \n = 0xA, which is also the keycode for Ctrl+J. The only reason we get
                    // newlines as input is because the terminal converts \r into \n for us. When we
                    // enter raw mode, we disable that, so \n no longer has any meaning - it's better to
                    // use Ctrl+J. Waiting to handle it here means it gets picked up later
                    '\n' if !raw_mode => KeyCode::Enter,
                    '\t' => {
                        if modifiers.contains(&KeyModifier::Shift) {
                            KeyCode::BackTab
                        } else {
                            KeyCode::Tab
                        }
                    }
                    '\x7F' => KeyCode::Backspace,
                    _ => {
                        if modifiers.contains(&KeyModifier::Shift) {
                            c = c.to_uppercase().next().expect("an upper case char");
                        }
                        KeyCode::Char(c)
                    }
                },
                KeyEventStates::default(),
            )
        } else {
            return Err("invalid char code point for CSI encoded key");
        }
    };

    if let KeyCode::Modifier(modifier_keycode) = keycode {
        match modifier_keycode {
            ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt => {
                modifiers.insert(KeyModifier::Alt);
            }
            ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => {
                modifiers.insert(KeyModifier::Control);
            }
            ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift => {
                modifiers.insert(KeyModifier::Shift);
            }
            ModifierKeyCode::LeftSuper | ModifierKeyCode::RightSuper => {
                modifiers.insert(KeyModifier::Super);
            }
            ModifierKeyCode::LeftHyper | ModifierKeyCode::RightHyper => {
                modifiers.insert(KeyModifier::Hyper);
            }
            ModifierKeyCode::LeftMeta | ModifierKeyCode::RightMeta => {
                modifiers.insert(KeyModifier::Meta);
            }
            _ => {}
        }
    }
    state.extend(state_from_keycode);

    let input_event = Event::Key(KeyEvent::new_with_kind_and_state(
        keycode, modifiers, kind, state,
    ));

    Ok(Some(input_event))
}

pub(crate) fn parse_csi_special_key_code<A: AppEvent>(buffer: &[u8]) -> Result<Option<Event<A>>> {
    assert!(buffer.starts_with(b"\x1B[")); // ESC [
    assert!(buffer.ends_with(b"~"));

    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| "Could not parse utf-8 for CSI special key")?;
    let mut split = s.split(';');

    // This CSI sequence can be a list of semicolon-separated numbers.
    let first = next_parsed::<u8>(&mut split)?;

    let modifiers = if let Ok(modifier_mask) = next_parsed::<u8>(&mut split) {
        parse_modifiers(modifier_mask)
    } else {
        KeyModifiers::default()
    };

    let keycode = match first {
        1 | 7 => KeyCode::Home,
        2 => KeyCode::Insert,
        3 => KeyCode::Delete,
        4 | 8 => KeyCode::End,
        5 => KeyCode::PageUp,
        6 => KeyCode::PageDown,
        v @ 11..=15 => KeyCode::F(v - 10),
        v @ 17..=21 => KeyCode::F(v - 11),
        v @ 23..=26 => KeyCode::F(v - 12),
        v @ 28..=29 => KeyCode::F(v - 15),
        v @ 31..=34 => KeyCode::F(v - 17),
        _ => return Err("Invalid CSI special key code"),
    };

    let input_event = Event::Key(KeyEvent::new(keycode, modifiers));

    Ok(Some(input_event))
}

pub(crate) fn parse_csi_rxvt_mouse<A: AppEvent>(buffer: &[u8]) -> Result<Option<Event<A>>> {
    // rxvt mouse encoding:
    // ESC [ Cb ; Cx ; Cy ; M

    assert!(buffer.starts_with(b"\x1B[")); // ESC [
    assert!(buffer.ends_with(b"M"));

    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| "Could not parse utf-8 for CSI rxvt mouse")?;
    let mut split = s.split(';');

    let cb = next_parsed::<u8>(&mut split)?
        .checked_sub(32)
        .ok_or("Small CB for CSI rxvt mouse")?;
    let (kind, modifiers) = parse_cb(cb)?;

    let cx = next_parsed::<u16>(&mut split)? - 1;
    let cy = next_parsed::<u16>(&mut split)? - 1;

    Ok(Some(Event::Mouse(MouseEvent {
        kind,
        column: cx,
        row: cy,
        modifiers,
    })))
}

pub(crate) fn parse_csi_normal_mouse<A: AppEvent>(buffer: &[u8]) -> Result<Option<Event<A>>> {
    // Normal mouse encoding: ESC [ M CB Cx Cy (6 characters only).

    assert!(buffer.starts_with(b"\x1B[M")); // ESC [ M

    if buffer.len() < 6 {
        return Ok(None);
    }

    let cb = buffer[3]
        .checked_sub(32)
        .ok_or("Invalid CSI normal mouse - ESC [ M CB Cx Cy - small CB")?;

    let (kind, modifiers) = parse_cb(cb)?;

    // See http://www.xfree86.org/current/ctlseqs.html#Mouse%20Tracking
    // The upper left character position on the terminal is denoted as 1,1.
    // Subtract 1 to keep it synced with cursor
    let cx = u16::from(buffer[4].saturating_sub(32)) - 1;
    let cy = u16::from(buffer[5].saturating_sub(32)) - 1;

    Ok(Some(Event::Mouse(MouseEvent {
        kind,
        column: cx,
        row: cy,
        modifiers,
    })))
}

pub(crate) fn parse_csi_sgr_mouse<A: AppEvent>(buffer: &[u8]) -> Result<Option<Event<A>>> {
    // ESC [ < Cb ; Cx ; Cy (;) (M or m)

    assert!(buffer.starts_with(b"\x1B[<")); // ESC [ <

    if !buffer.ends_with(b"m") && !buffer.ends_with(b"M") {
        return Ok(None);
    }

    let s = std::str::from_utf8(&buffer[3..buffer.len() - 1])
        .map_err(|_| "Could not parse utf-8 for CSI sgr mouse")?;
    let mut split = s.split(';');

    let cb = next_parsed::<u8>(&mut split)?;
    let (kind, modifiers) = parse_cb(cb)?;

    // See http://www.xfree86.org/current/ctlseqs.html#Mouse%20Tracking
    // The upper left character position on the terminal is denoted as 1,1.
    // Subtract 1 to keep it synced with cursor
    let cx = next_parsed::<u16>(&mut split)? - 1;
    let cy = next_parsed::<u16>(&mut split)? - 1;

    // When button 3 in Cb is used to represent mouse release, you can't tell which button was
    // released. SGR mode solves this by having the sequence end with a lowercase m if it's a
    // button release and an uppercase M if it's a button press.
    //
    // We've already checked that the last character is a lowercase or uppercase M at the start of
    // this function, so we just need one if.
    let kind = if buffer.last() == Some(&b'm') {
        match kind {
            MouseEventKind::Down(button) => MouseEventKind::Up(button),
            other => other,
        }
    } else {
        kind
    };

    Ok(Some(Event::Mouse(MouseEvent {
        kind,
        column: cx,
        row: cy,
        modifiers,
    })))
}

/// Cb is the byte of a mouse input that contains the button being used, the key modifiers being
/// held and whether the mouse is dragging or not.
///
/// Bit layout of cb, from low to high:
///
/// - button number
/// - button number
/// - shift
/// - meta (alt)
/// - control
/// - mouse is dragging
/// - button number
/// - button number
fn parse_cb(cb: u8) -> Result<(MouseEventKind, KeyModifiers)> {
    let button_number = (cb & 0b0000_0011) | ((cb & 0b1100_0000) >> 4);
    let dragging = cb & 0b0010_0000 == 0b0010_0000;

    let kind = match (button_number, dragging) {
        (0, false) => MouseEventKind::Down(MouseButton::Left),
        (1, false) => MouseEventKind::Down(MouseButton::Middle),
        (2, false) => MouseEventKind::Down(MouseButton::Right),
        (0, true) => MouseEventKind::Drag(MouseButton::Left),
        (1, true) => MouseEventKind::Drag(MouseButton::Middle),
        (2, true) => MouseEventKind::Drag(MouseButton::Right),
        (3, false) => MouseEventKind::Up(MouseButton::Left),
        (3, true) | (4, true) | (5, true) => MouseEventKind::Moved,
        (4, false) => MouseEventKind::ScrollUp,
        (5, false) => MouseEventKind::ScrollDown,
        // We do not support other buttons.
        _ => return Err("Unsupported CSI normal mouse buttons"),
    };

    let mut modifiers = KeyModifiers::default();

    if cb & 0b0000_0100 == 0b0000_0100 {
        modifiers.insert(KeyModifier::Shift);
    }
    if cb & 0b0000_1000 == 0b0000_1000 {
        modifiers.insert(KeyModifier::Alt);
    }
    if cb & 0b0001_0000 == 0b0001_0000 {
        modifiers.insert(KeyModifier::Control);
    }

    Ok((kind, modifiers))
}

pub(crate) fn parse_csi_bracketed_paste<A: AppEvent>(buffer: &[u8]) -> Result<Option<Event<A>>> {
    // ESC [ 2 0 0 ~ pasted text ESC 2 0 1 ~
    assert!(buffer.starts_with(b"\x1B[200~"));

    if !buffer.ends_with(b"\x1b[201~") {
        Ok(None)
    } else {
        let paste = String::from_utf8_lossy(&buffer[6..buffer.len() - 6]).to_string();
        Ok(Some(Event::Paste(paste)))
    }
}

pub(crate) fn parse_utf8_char(buffer: &[u8]) -> Result<Option<char>> {
    match std::str::from_utf8(buffer) {
        Ok(s) => {
            let ch = s.chars().next().ok_or("missing utf-8 char")?;

            Ok(Some(ch))
        }
        Err(_) => {
            // from_utf8 failed, but we have to check if we need more bytes for code point
            // and if all the bytes we have no are valid

            let required_bytes = match buffer[0] {
                // https://en.wikipedia.org/wiki/UTF-8#Description
                (0x00..=0x7F) => 1, // 0xxxxxxx
                (0xC0..=0xDF) => 2, // 110xxxxx 10xxxxxx
                (0xE0..=0xEF) => 3, // 1110xxxx 10xxxxxx 10xxxxxx
                (0xF0..=0xF7) => 4, // 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
                (0x80..=0xBF) | (0xF8..=0xFF) => return Err("invalid utf-8 char"),
            };

            // More than 1 byte, check them for 10xxxxxx pattern
            if required_bytes > 1 && buffer.len() > 1 {
                for byte in &buffer[1..] {
                    if byte & !0b0011_1111 != 0b1000_0000 {
                        return Err("invalid utf-8 char");
                    }
                }
            }

            if buffer.len() < required_bytes {
                // All bytes looks good so far, but we need more of them
                Ok(None)
            } else {
                Err("invalid utf-8 char")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::input::events::{KeyEventState, KeyModifiers, MouseButton, MouseEvent};

    use super::*;

    #[test]
    fn test_esc_key() {
        assert_eq!(
            parse_esc::<()>(b"\x1B", false).unwrap(),
            Some(Event::Key(KeyCode::Esc.into())),
        );
    }

    #[test]
    fn test_alt_key() {
        assert_eq!(
            parse_esc::<()>(b"\x1Bc", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                Some(KeyModifier::Alt)
            ))),
        );
    }

    #[test]
    fn test_alt_shift() {
        assert_eq!(
            parse_esc::<()>(b"\x1BH", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('H'),
                [KeyModifier::Alt, KeyModifier::Shift]
            ))),
        );
    }

    #[test]
    fn test_alt_ctrl() {
        assert_eq!(
            parse_esc::<()>(b"\x1B\x14", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('t'),
                [KeyModifier::Alt, KeyModifier::Control]
            ))),
        );
    }

    #[test]
    fn test_parse_esc_subsequent_calls() {
        // The main purpose of this test is to check that parse_esc passes the
        // correct slice on to the other parse_ functions.

        // parse_csi_cursor_position
        assert_eq!(
            parse_esc::<()>(b"\x1B[20;10R", false).unwrap(),
            Some(Event::CursorPosition(point(9, 19)))
        );

        // parse_csi
        assert_eq!(
            parse_esc::<()>(b"\x1B[D", false).unwrap(),
            Some(Event::Key(KeyCode::Left.into())),
        );

        // parse_csi_modifier_key_code
        assert_eq!(
            parse_esc::<()>(b"\x1B[2D", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Left,
                Some(KeyModifier::Shift)
            )))
        );

        // parse_csi_special_key_code
        assert_eq!(
            parse_esc::<()>(b"\x1B[3~", false).unwrap(),
            Some(Event::Key(KeyCode::Delete.into())),
        );

        // parse_csi_bracketed_paste
        assert_eq!(
            parse_esc::<()>(b"\x1B[200~on and on and on\x1B[201~", false).unwrap(),
            Some(Event::Paste("on and on and on".to_string())),
        );

        // parse_csi_rxvt_mouse
        assert_eq!(
            parse_esc::<()>(b"\x1B[32;30;40;M", false).unwrap(),
            Some(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 29,
                row: 39,
                modifiers: KeyModifiers::default(),
            }))
        );

        // parse_csi_normal_mouse
        assert_eq!(
            parse_esc::<()>(b"\x1B[M0\x60\x70", false).unwrap(),
            Some(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 63,
                row: 79,
                modifiers: Some(KeyModifier::Control).into_iter().collect(),
            }))
        );

        // parse_csi_sgr_mouse
        assert_eq!(
            parse_esc::<()>(b"\x1B[<0;20;10;M", false).unwrap(),
            Some(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 19,
                row: 9,
                modifiers: KeyModifiers::default(),
            }))
        );

        // parse_utf8_char
        assert_eq!(parse_utf8_char("Ž".as_bytes()).unwrap(), Some('Ž'));
        assert_eq!(
            parse_key::<()>('Ž', false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('Ž'),
                Some(KeyModifier::Shift)
            ))),
        );
    }

    #[test]
    fn test_parse_key_control_char() {
        assert_eq!(
            parse_key::<()>('\t', false).unwrap(),
            Some(Event::Key(KeyCode::Tab.into())),
        );
    }

    #[test]
    fn test_parse_csi_cursor_position() {
        assert_eq!(
            parse_csi_cursor_position::<()>(b"\x1B[20;10R").unwrap(),
            Some(Event::CursorPosition(point(9, 19)))
        );
    }

    #[test]
    fn test_parse_csi() {
        assert_eq!(
            parse_csi::<()>(b"\x1B[D", false).unwrap(),
            Some(Event::Key(KeyCode::Left.into())),
        );
    }

    #[test]
    fn test_parse_csi_modifier_key_code() {
        assert_eq!(
            parse_csi_modifier_key_code::<()>(b"\x1B[2D").unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Left,
                Some(KeyModifier::Shift)
            ))),
        );
    }

    #[test]
    fn test_parse_csi_special_key_code() {
        assert_eq!(
            parse_csi_special_key_code::<()>(b"\x1B[3~").unwrap(),
            Some(Event::Key(KeyCode::Delete.into())),
        );
    }

    #[test]
    fn test_parse_csi_special_key_code_multiple_values_not_supported() {
        assert_eq!(
            parse_csi_special_key_code::<()>(b"\x1B[3;2~").unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Delete,
                Some(KeyModifier::Shift)
            ))),
        );
    }

    #[test]
    fn test_parse_csi_bracketed_paste() {
        //
        assert_eq!(
            parse_esc::<()>(b"\x1B[200~o", false).unwrap(),
            None,
            "A partial bracketed paste isn't parsed"
        );
        assert_eq!(
            parse_esc::<()>(b"\x1B[200~o\x1B[2D", false).unwrap(),
            None,
            "A partial bracketed paste containing another escape code isn't parsed"
        );
        assert_eq!(
            parse_esc::<()>(b"\x1B[200~o\x1B[2D\x1B[201~", false).unwrap(),
            Some(Event::Paste("o\x1B[2D".to_string()))
        );
    }

    #[test]
    fn test_parse_csi_focus() {
        assert_eq!(
            parse_csi::<()>(b"\x1B[O", false).unwrap(),
            Some(Event::Focus(false))
        );
    }

    #[test]
    fn test_parse_csi_rxvt_mouse() {
        assert_eq!(
            parse_csi_rxvt_mouse::<()>(b"\x1B[32;30;40;M").unwrap(),
            Some(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 29,
                row: 39,
                modifiers: KeyModifiers::default(),
            }))
        );
    }

    #[test]
    fn test_parse_csi_normal_mouse() {
        assert_eq!(
            parse_csi_normal_mouse::<()>(b"\x1B[M0\x60\x70").unwrap(),
            Some(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 63,
                row: 79,
                modifiers: Some(KeyModifier::Control).into_iter().collect(),
            }))
        );
    }

    #[test]
    fn test_parse_csi_sgr_mouse() {
        assert_eq!(
            parse_csi_sgr_mouse::<()>(b"\x1B[<0;20;10;M").unwrap(),
            Some(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 19,
                row: 9,
                modifiers: KeyModifiers::default(),
            }))
        );
        assert_eq!(
            parse_csi_sgr_mouse::<()>(b"\x1B[<0;20;10M").unwrap(),
            Some(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 19,
                row: 9,
                modifiers: KeyModifiers::default(),
            }))
        );
        assert_eq!(
            parse_csi_sgr_mouse::<()>(b"\x1B[<0;20;10;m").unwrap(),
            Some(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 19,
                row: 9,
                modifiers: KeyModifiers::default(),
            }))
        );
        assert_eq!(
            parse_csi_sgr_mouse::<()>(b"\x1B[<0;20;10m").unwrap(),
            Some(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 19,
                row: 9,
                modifiers: KeyModifiers::default(),
            }))
        );
    }

    #[test]
    fn test_utf8() {
        // https://www.php.net/manual/en/reference.pcre.pattern.modifiers.php#54805

        // 'Valid ASCII' => "a",
        assert_eq!(parse_utf8_char(b"a").unwrap(), Some('a'),);

        // 'Valid 2 Octet Sequence' => "\xc3\xb1",
        assert_eq!(parse_utf8_char(&[0xC3, 0xB1]).unwrap(), Some('ñ'),);

        // 'Invalid 2 Octet Sequence' => "\xc3\x28",
        assert!(parse_utf8_char(&[0xC3, 0x28]).is_err());

        // 'Invalid Sequence Identifier' => "\xa0\xa1",
        assert!(parse_utf8_char(&[0xA0, 0xA1]).is_err());

        // 'Valid 3 Octet Sequence' => "\xe2\x82\xa1",
        assert_eq!(
            parse_utf8_char(&[0xE2, 0x81, 0xA1]).unwrap(),
            Some('\u{2061}'),
        );

        // 'Invalid 3 Octet Sequence (in 2nd Octet)' => "\xe2\x28\xa1",
        assert!(parse_utf8_char(&[0xE2, 0x28, 0xA1]).is_err());

        // 'Invalid 3 Octet Sequence (in 3rd Octet)' => "\xe2\x82\x28",
        assert!(parse_utf8_char(&[0xE2, 0x82, 0x28]).is_err());

        // 'Valid 4 Octet Sequence' => "\xf0\x90\x8c\xbc",
        assert_eq!(
            parse_utf8_char(&[0xF0, 0x90, 0x8C, 0xBC]).unwrap(),
            Some('𐌼'),
        );

        // 'Invalid 4 Octet Sequence (in 2nd Octet)' => "\xf0\x28\x8c\xbc",
        assert!(parse_utf8_char(&[0xF0, 0x28, 0x8C, 0xBC]).is_err());

        // 'Invalid 4 Octet Sequence (in 3rd Octet)' => "\xf0\x90\x28\xbc",
        assert!(parse_utf8_char(&[0xF0, 0x90, 0x28, 0xBC]).is_err());

        // 'Invalid 4 Octet Sequence (in 4th Octet)' => "\xf0\x28\x8c\x28",
        assert!(parse_utf8_char(&[0xF0, 0x28, 0x8C, 0x28]).is_err());
    }

    #[test]
    fn test_parse_char_event_lowercase() {
        assert_eq!(
            parse_key::<()>('c', false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::default()
            ))),
        );
    }

    #[test]
    fn test_parse_char_event_uppercase() {
        assert_eq!(
            parse_key::<()>('C', false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('C'),
                Some(KeyModifier::Shift)
            ))),
        );
    }

    #[test]
    fn test_parse_basic_csi_u_encoded_key_code() {
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::default()
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97;2u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('A'),
                Some(KeyModifier::Shift)
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97;7u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                [KeyModifier::Alt, KeyModifier::Control]
            ))),
        );
    }

    #[test]
    fn test_parse_basic_csi_u_encoded_key_code_special_keys() {
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[13u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::default()
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[27u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Esc,
                KeyModifiers::default()
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[57358u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::CapsLock,
                KeyModifiers::default()
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[57376u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::F(13),
                KeyModifiers::default()
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[57428u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Media(MediaKeyCode::Play),
                KeyModifiers::default()
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[57441u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Modifier(ModifierKeyCode::LeftShift),
                Some(KeyModifier::Shift)
            ))),
        );
    }

    #[test]
    fn test_parse_csi_u_encoded_keypad_code() {
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[57399u", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind_and_state(
                KeyCode::Char('0'),
                KeyModifiers::default(),
                KeyEventKind::Press,
                Some(KeyEventState::Keypad),
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[57419u", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind_and_state(
                KeyCode::Up,
                KeyModifiers::default(),
                KeyEventKind::Press,
                Some(KeyEventState::Keypad),
            ))),
        );
    }

    #[test]
    fn test_parse_csi_u_encoded_key_code_with_types() {
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97;1u", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::default(),
                KeyEventKind::Press,
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97;1:1u", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::default(),
                KeyEventKind::Press,
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97;5:1u", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('a'),
                Some(KeyModifier::Control),
                KeyEventKind::Press,
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97;1:2u", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::default(),
                KeyEventKind::Repeat,
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97;1:3u", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::default(),
                KeyEventKind::Release,
            ))),
        );
    }

    #[test]
    fn test_parse_csi_u_encoded_key_code_has_modifier_on_modifier_press() {
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[57449u", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Modifier(ModifierKeyCode::RightAlt),
                Some(KeyModifier::Alt),
                KeyEventKind::Press,
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[57449;3:3u", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Modifier(ModifierKeyCode::RightAlt),
                Some(KeyModifier::Alt),
                KeyEventKind::Release,
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[57450u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Modifier(ModifierKeyCode::RightSuper),
                Some(KeyModifier::Super),
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[57451u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Modifier(ModifierKeyCode::RightHyper),
                Some(KeyModifier::Hyper),
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[57452u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Modifier(ModifierKeyCode::RightMeta),
                Some(KeyModifier::Meta),
            ))),
        );
    }

    #[test]
    fn test_parse_csi_u_encoded_key_code_with_extra_modifiers() {
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97;9u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                Some(KeyModifier::Super)
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97;17u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                Some(KeyModifier::Hyper)
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97;33u", false).unwrap(),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                Some(KeyModifier::Meta)
            ))),
        );
    }

    #[test]
    fn test_parse_csi_u_encoded_key_code_with_extra_state() {
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[97;65u", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind_and_state(
                KeyCode::Char('a'),
                KeyModifiers::default(),
                KeyEventKind::Press,
                Some(KeyEventState::CapsLock),
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code::<()>(b"\x1B[49;129u", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind_and_state(
                KeyCode::Char('1'),
                KeyModifiers::default(),
                KeyEventKind::Press,
                Some(KeyEventState::NumLock),
            ))),
        );
    }

    #[test]
    fn test_parse_csi_special_key_code_with_types() {
        assert_eq!(
            parse_esc::<()>(b"\x1B[;1:3B", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Down,
                KeyModifiers::default(),
                KeyEventKind::Release,
            ))),
        );
        assert_eq!(
            parse_esc::<()>(b"\x1B[1;1:3B", false).unwrap(),
            Some(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Down,
                KeyModifiers::default(),
                KeyEventKind::Release,
            ))),
        );
    }
}
