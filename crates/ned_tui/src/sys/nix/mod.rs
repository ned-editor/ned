//! unix tty control and raw mode lifecycle management.

pub mod tty;

use super::Control;
use crate::{input::DefaultInput, output::DefaultOutput, prelude::Point, Terminal};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::{fs, io, sync::Arc};

/// default control implementation on unix.
pub type DefaultControl = NixTty;
/// control output file type.
pub type ControlOut = fs::File;
/// control input file type.
pub type ControlIn = fs::File;

/// Creates the default terminal implementation for Unix.
pub(crate) fn terminal() -> io::Result<Terminal<DefaultControl, DefaultInput, DefaultOutput>> {
    let control = get_tty()?;
    Ok(Terminal::new(
        control,
        DefaultInput,
        DefaultOutput::default(),
    ))
}

/// tries to get terminal tty from stdin, stdout, or /dev/tty.
pub fn get_tty() -> io::Result<DefaultControl> {
    let input = if tty::is_tty(libc::STDIN_FILENO) {
        unsafe {
            let dup_fd = libc::dup(libc::STDIN_FILENO);
            if dup_fd >= 0 {
                Some(fs::File::from_raw_fd(dup_fd))
            } else {
                None
            }
        }
    } else {
        None
    }
    .or_else(|| tty::open_tty().ok())
    .ok_or_else(|| io::Error::other("no tty input was found"))?;

    let output = if tty::is_tty(libc::STDOUT_FILENO) {
        unsafe {
            let dup_fd = libc::dup(libc::STDOUT_FILENO);
            if dup_fd >= 0 {
                Some(fs::File::from_raw_fd(dup_fd))
            } else {
                None
            }
        }
    } else {
        None
    }
    .or_else(|| tty::open_tty().ok())
    .ok_or_else(|| io::Error::other("no tty output was found"))?;

    NixTty::try_from((input, output))
}

/// wraps tty handles and restores terminal attributes on drop.
pub struct NixTty {
    /// Reset guard that restores terminal attributes on drop.
    reset: Arc<Reset>,
}

impl TryFrom<(fs::File, fs::File)> for NixTty {
    type Error = io::Error;

    fn try_from(value: (fs::File, fs::File)) -> Result<Self, Self::Error> {
        let (input, output) = value;
        let state = tty::get_tty_attributes(input.as_raw_fd())
            .or_else(|_| tty::get_tty_attributes(output.as_raw_fd()))?;

        Ok(NixTty {
            reset: Arc::new(Reset {
                state,
                input,
                output,
            }),
        })
    }
}

impl Control for NixTty {
    fn enable_raw_mode(&self) -> io::Result<()> {
        tty::enable_raw(self.reset.input.as_raw_fd())
            .or_else(|_| tty::enable_raw(self.reset.output.as_raw_fd()))
    }

    fn get_size(&self) -> io::Result<Point> {
        tty::size_from_fd(self.reset.output.as_raw_fd())
            .or_else(|_| tty::size_from_fd(self.reset.input.as_raw_fd()))
    }

    fn is_tty(&self) -> bool {
        tty::is_tty(self.reset.output.as_raw_fd()) || tty::is_tty(self.reset.input.as_raw_fd())
    }

    fn is_raw(&self) -> bool {
        tty::is_raw(self.reset.output.as_raw_fd())
            .or_else(|_| tty::is_raw(self.reset.input.as_raw_fd()))
            .unwrap_or_default()
    }

    fn is_ansi(&self) -> bool {
        tty::is_raw(self.reset.output.as_raw_fd())
            .or_else(|_| tty::is_raw(self.reset.input.as_raw_fd()))
            .unwrap_or_default()
    }

    fn output(&self) -> &ControlOut {
        &self.reset.output
    }

    fn input(&self) -> &ControlIn {
        &self.reset.input
    }

    fn clone(&self) -> Self {
        Self {
            reset: self.reset.clone(),
        }
    }
}

/// Terminal state snapshot and file handles for reset on drop.
struct Reset {
    /// Saved terminal attributes.
    state: libc::termios,
    /// Input file handle.
    input: fs::File,
    /// Output file handle.
    output: fs::File,
}

impl Drop for Reset {
    fn drop(&mut self) {
        let _ = tty::set_tty_attributes(self.input.as_raw_fd(), &self.state);
        let _ = tty::set_tty_attributes(self.output.as_raw_fd(), &self.state);
    }
}
