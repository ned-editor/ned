//! unix tty attribute and size queries using libc.

use crate::prelude::{Point, point};
use std::fs;
use std::io;
use std::os::unix::io::RawFd;

/// checks whether the file descriptor refers to a terminal.
pub fn is_tty(fd: RawFd) -> bool {
    unsafe { libc::isatty(fd) == 1 }
}

/// gets current termios attributes for the file descriptor.
pub fn get_tty_attributes(fd: RawFd) -> io::Result<libc::termios> {
    unsafe {
        let mut tios: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(fd, &mut tios) != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(tios)
    }
}

/// sets termios attributes for the file descriptor immediately.
pub fn set_tty_attributes(fd: RawFd, tios: &libc::termios) -> io::Result<()> {
    unsafe {
        if libc::tcsetattr(fd, libc::TCSANOW, tios) != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

/// checks whether the file descriptor is currently in raw mode.
pub fn is_raw(fd: RawFd) -> io::Result<bool> {
    let tios = get_tty_attributes(fd)?;
    let is_echo = (tios.c_lflag & libc::ECHO) != 0;
    let is_icanon = (tios.c_lflag & libc::ICANON) != 0;
    Ok(!is_echo && !is_icanon)
}

/// switches terminal into raw mode.
pub fn enable_raw(fd: RawFd) -> io::Result<()> {
    unsafe {
        let mut tios = get_tty_attributes(fd)?;
        libc::cfmakeraw(&mut tios);
        set_tty_attributes(fd, &tios)?;
        Ok(())
    }
}

/// opens `/dev/tty` for terminal control.
pub fn open_tty() -> io::Result<fs::File> {
    fs::File::open("/dev/tty")
}

/// queries terminal dimensions (cols, rows) via ioctl TIOCGWINSZ.
pub fn size_from_fd(fd: RawFd) -> io::Result<Point> {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(point(ws.ws_col, ws.ws_row))
    }
}
