#![allow(unsafe_code)]
//! yokio-based terminal input watcher with zero-dependency ansi tokenization.

use super::events::{AppEvent, Event};
use super::parser::*;
use crate::prelude::Point;
use crate::sys::Control;
use std::cell::{Cell, RefCell};
use std::fs;
use std::io::{self, Read};
use std::rc::Rc;
use yokio::io::{Interest, Mode, Watch};

#[cfg(unix)]
static RESIZE_PIPE_WRITE_FD: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

#[cfg(unix)]
extern "C" fn sigwinch_handler(_: libc::c_int) {
    let fd = RESIZE_PIPE_WRITE_FD.load(std::sync::atomic::Ordering::Relaxed);
    if fd >= 0 {
        let b = [1u8];
        unsafe {
            libc::write(fd, b.as_ptr() as *const libc::c_void, 1);
        }
    }
}

/// Helper to query current terminal window size across platforms.
pub fn query_terminal_size() -> io::Result<Point> {
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        if let Ok(file) = fs::File::open("/dev/tty") {
            if let Ok(pt) = crate::sys::tty::size_from_fd(file.as_raw_fd()) {
                return Ok(pt);
            }
        }
        crate::sys::tty::size_from_fd(libc::STDOUT_FILENO)
            .or_else(|_| crate::sys::tty::size_from_fd(libc::STDIN_FILENO))
    }
    #[cfg(windows)]
    {
        use crate::sys::win::console;
        if let Ok(file) = console::open_console(true) {
            console::output::size_from_handle(&file)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "no console handle"))
        }
    }
}

/// watcher handle for terminal input events.
pub struct InputWatcher {
    _watch: Watch,
    #[cfg(unix)]
    _resize_watch: Option<Watch>,
}

/// starts watching terminal input using yokio event loop.
pub fn watch_terminal_input<A: AppEvent, F: FnMut(io::Result<Event<A>>) + 'static>(
    _control: &(impl Control + ?Sized),
    raw_mode: bool,
    on_event: F,
) -> io::Result<InputWatcher> {
    let on_event = Rc::new(RefCell::new(on_event));
    let initial_size = query_terminal_size().unwrap_or_default();
    let last_size = Rc::new(Cell::new(initial_size));

    #[cfg(unix)]
    let reader = _control
        .input()
        .try_clone()
        .or_else(|_| fs::File::open("/dev/tty"))
        .or_else(|_| unsafe {
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

    let on_event_input = Rc::clone(&on_event);
    let last_size_input = Rc::clone(&last_size);

    let watch = yokio::watcher(&reader)
        .interest(Interest::READABLE)
        .mode(Mode::Edge)
        .start(move |ready| {
            if ready.is_readable() {
                // Check if window size changed (essential for Windows and fallback on Unix)
                if let Ok(current_size) = query_terminal_size() {
                    if current_size.col > 0
                        && current_size.row > 0
                        && current_size != last_size_input.get()
                    {
                        last_size_input.set(current_size);
                        on_event_input.borrow_mut()(Ok(Event::Resize(current_size)));
                    }
                }

                loop {
                    match read_copy.read(&mut raw_buf) {
                        Ok(0) => {
                            on_event_input.borrow_mut()(Ok(Event::InputClosed));
                            break;
                        }
                        Ok(n) => {
                            pending.extend_from_slice(&raw_buf[..n]);
                            let mut cb = |ev| on_event_input.borrow_mut()(ev);
                            parse_pending_buffer(&mut pending, raw_mode, &mut cb);
                        }
                        Err(read_err) => {
                            if read_err.kind() == io::ErrorKind::WouldBlock {
                                if pending.len() == 1 && pending[0] == 0x1B {
                                    pending.clear();
                                    on_event_input.borrow_mut()(Ok(Event::Key(
                                        super::events::KeyCode::Esc.into(),
                                    )));
                                }
                                break;
                            }
                            on_event_input.borrow_mut()(Err(read_err));
                            break;
                        }
                    }
                }
            }
        })
        .map_err(|e| io::Error::other(e.to_string()))?;

    #[cfg(unix)]
    let resize_watch = {
        use std::os::unix::io::FromRawFd;
        let mut pipe_fds = [0i32; 2];
        let res = unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
        if res == 0 {
            unsafe {
                let flags0 = libc::fcntl(pipe_fds[0], libc::F_GETFL);
                if flags0 != -1 {
                    libc::fcntl(pipe_fds[0], libc::F_SETFL, flags0 | libc::O_NONBLOCK);
                }
                let flags1 = libc::fcntl(pipe_fds[1], libc::F_GETFL);
                if flags1 != -1 {
                    libc::fcntl(pipe_fds[1], libc::F_SETFL, flags1 | libc::O_NONBLOCK);
                }
                RESIZE_PIPE_WRITE_FD.store(pipe_fds[1], std::sync::atomic::Ordering::Relaxed);
                libc::signal(libc::SIGWINCH, sigwinch_handler as *const () as usize);
            }
            let resize_file = unsafe { fs::File::from_raw_fd(pipe_fds[0]) };
            let mut resize_copy = resize_file.try_clone().ok();
            let on_event_resize = Rc::clone(&on_event);
            let last_size_resize = Rc::clone(&last_size);

            yokio::watcher(&resize_file)
                .interest(Interest::READABLE)
                .mode(Mode::Edge)
                .start(move |_ready| {
                    if let Some(ref mut r) = resize_copy {
                        let mut buf = [0u8; 64];
                        while let Ok(n) = r.read(&mut buf) {
                            if n == 0 {
                                break;
                            }
                        }
                    }
                    if let Ok(current_size) = query_terminal_size() {
                        if current_size.col > 0
                            && current_size.row > 0
                            && current_size != last_size_resize.get()
                        {
                            last_size_resize.set(current_size);
                            on_event_resize.borrow_mut()(Ok(Event::Resize(current_size)));
                        }
                    }
                })
                .ok()
        } else {
            None
        }
    };

    Ok(InputWatcher {
        _watch: watch,
        #[cfg(unix)]
        _resize_watch: resize_watch,
    })
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
            let text_end = buffer
                .iter()
                .position(|&b| b == 0x1B)
                .unwrap_or(buffer.len());
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
