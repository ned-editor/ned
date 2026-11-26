//! yokio-based terminal input watcher with zero-dependency ansi tokenization.

use super::events::{AppEvent, Event};
use super::parser::*;
use crate::sys::Control;
use std::fs;
use std::io::{self, Read};
use yokio::io::{Interest, Mode, Watch};

/// watcher handle for terminal input events.
pub struct InputWatcher {
    _watch: Watch,
}

/// starts watching terminal input using yokio event loop.
pub fn watch_terminal_input<A: AppEvent, F: FnMut(io::Result<Event<A>>) + 'static>(
    _control: &(impl Control + ?Sized),
    raw_mode: bool,
    mut on_event: F,
) -> io::Result<InputWatcher> {
    #[cfg(unix)]
    let reader = fs::File::open("/dev/tty").or_else(|_| unsafe {
        let fd = libc::dup(libc::STDIN_FILENO);
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            use std::os::unix::io::FromRawFd;
            Ok(fs::File::from_raw_fd(fd))
        }
    })?;

    #[cfg(windows)]
    let reader = _control.input().clone();

    #[cfg(unix)]
    unsafe {
        use std::os::unix::io::AsRawFd;
        let fd = reader.as_raw_fd();
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags != -1 {
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
    }

    let mut read_copy = reader.try_clone()?;
    let mut raw_buf = [0u8; 1024];
    let mut pending = Vec::new();

    let watch = yokio::watcher(&reader)
        .interest(Interest::READABLE)
        .mode(Mode::Edge)
        .start(move |ready| {
            if ready.is_readable() {
                loop {
                    match read_copy.read(&mut raw_buf) {
                        Ok(0) => {
                            on_event(Ok(Event::InputClosed));
                            break;
                        }
                        Ok(n) => {
                            pending.extend_from_slice(&raw_buf[..n]);
                            parse_pending_buffer(&mut pending, raw_mode, &mut on_event);
                        }
                        Err(read_err) => {
                            if read_err.kind() == io::ErrorKind::WouldBlock {
                                break;
                            }
                            on_event(Err(read_err));
                            break;
                        }
                    }
                }
            }
        })
        .map_err(|e| io::Error::other(e.to_string()))?;

    Ok(InputWatcher { _watch: watch })
}

/// parses pending buffer by discovering ansi boundaries and invoking callbacks.
fn parse_pending_buffer<A: AppEvent, F: FnMut(io::Result<Event<A>>)>(
    buffer: &mut Vec<u8>,
    raw_mode: bool,
    on_event: &mut F,
) {
    while !buffer.is_empty() {
        if buffer[0] == 0x1B {
            if buffer.len() == 1 {
                // incomplete escape sequence, wait for next bytes
                break;
            }

            match buffer[1] {
                b'[' => {
                    // CSI sequence: ESC [ ... <final-byte in 0x40..=0x7E>
                    let mut final_byte_idx = None;
                    for (i, &b) in buffer.iter().enumerate().skip(2) {
                        if (0x40..=0x7E).contains(&b) {
                            final_byte_idx = Some(i);
                            break;
                        }
                    }

                    if let Some(end_idx) = final_byte_idx {
                        let csi_slice = &buffer[..=end_idx];
                        if let Ok(Some(ev)) = parse_csi::<A>(csi_slice, raw_mode) {
                            on_event(Ok(ev));
                        } else {
                            on_event(Ok(Event::Unknown(csi_slice.to_vec())));
                        }
                        buffer.drain(..=end_idx);
                    } else {
                        // incomplete CSI sequence, wait for final byte
                        break;
                    }
                }
                b'O' => {
                    // SS3 sequence: ESC O <byte>
                    if buffer.len() >= 3 {
                        let esc_slice = &buffer[..3];
                        if let Ok(Some(ev)) = parse_esc::<A>(esc_slice, raw_mode) {
                            on_event(Ok(ev));
                        } else {
                            on_event(Ok(Event::Unknown(esc_slice.to_vec())));
                        }
                        buffer.drain(..3);
                    } else {
                        break;
                    }
                }
                _ => {
                    // other escape sequence (alt+key or double esc)
                    if let Ok(Some(ev)) = parse_esc::<A>(buffer, raw_mode) {
                        on_event(Ok(ev));
                        buffer.drain(..2);
                    } else {
                        let unk = buffer.remove(0);
                        on_event(Ok(Event::Unknown(vec![unk])));
                    }
                }
            }
        } else {
            // regular text bytes up to next ESC
            let text_end = buffer.iter().position(|&b| b == 0x1B).unwrap_or(buffer.len());
            let text_slice = &buffer[..text_end];

            match std::str::from_utf8(text_slice) {
                Ok(s) => {
                    for c in s.chars() {
                        if let Ok(Some(ev)) = parse_key::<A>(c, raw_mode) {
                            on_event(Ok(ev));
                        }
                    }
                    buffer.drain(..text_end);
                }
                Err(err) => {
                    let valid_up_to = err.valid_up_to();
                    if valid_up_to > 0 {
                        if let Ok(s) = std::str::from_utf8(&text_slice[..valid_up_to]) {
                            for c in s.chars() {
                                if let Ok(Some(ev)) = parse_key::<A>(c, raw_mode) {
                                    on_event(Ok(ev));
                                }
                            }
                        }
                        buffer.drain(..valid_up_to);
                    } else {
                        // invalid byte at start, discard single byte
                        let bad_byte = buffer.remove(0);
                        on_event(Ok(Event::Unknown(vec![bad_byte])));
                    }
                }
            }
        }
    }
}
