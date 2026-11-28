//! terminal initialization and configuration facade.

use crate::{
    input::events::AppEvent,
    input::DefaultInput,
    output::{
        command::{TerminalCommander, TerminalRequest},
        DefaultOutput,
    },
    sys::{Control, DefaultControl},
    Termit,
};
use std::io::{self, Read};

/// the terminal facilitator representing control, input, and output handles.
pub struct Terminal<C, I, O> {
    /// tty control interface for terminal attributes and sizing.
    pub control: C,
    /// input stream handle.
    pub input: I,
    /// output stream handle.
    pub output: O,
}

impl<'a, C, I, O> Terminal<C, I, O>
where
    C: Control + 'a,
    I: Read + 'a,
    O: io::Write + 'a,
{
    /// converts terminal into a termit instance for ui rendering.
    pub fn into_termit<A: AppEvent>(self) -> Termit<'a, A> {
        Termit::from(self)
    }
}

/// No-op terminal I/O handle for disconnected or headless environments.
#[derive(Default, Copy, Clone)]
struct NoIo;

impl Default for Terminal<NoIo, NoIo, NoIo> {
    fn default() -> Self {
        Self {
            control: NoIo,
            input: NoIo,
            output: NoIo,
        }
    }
}

impl Terminal<DefaultControl, DefaultInput, DefaultOutput> {
    /// tries to get default system terminal configuration.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if no tty is available.
    pub fn try_system_default() -> io::Result<Self> {
        crate::sys::terminal()
    }
}

impl<C, I, O> Terminal<C, I, O> {
    /// creates a new terminal instance with specified handles.
    pub fn new(control: C, input: I, output: O) -> Terminal<C, I, O> {
        Terminal {
            control,
            input,
            output,
        }
    }
}

impl<C, I, O> Terminal<C, I, O>
where
    O: io::Write,
{
    /// changes the controlling tty.
    pub fn controlling<C2>(self, control: C2) -> Terminal<C2, I, O> {
        let Self {
            control: _,
            input,
            output,
        } = self;
        Terminal {
            control,
            input,
            output,
        }
    }

    /// changes the input source.
    pub fn reading<I2>(self, input: I2) -> Terminal<C, I2, O> {
        let Self {
            control,
            input: _,
            output,
        } = self;
        Terminal {
            control,
            input,
            output,
        }
    }

    /// changes the output sink.
    pub fn writing<O2>(self, output: O2) -> Terminal<C, I, O2>
    where
        O2: io::Write,
    {
        let Self {
            control,
            input,
            output: _,
        } = self;
        Terminal {
            control,
            input,
            output,
        }
    }
}

impl<C, I, O> TerminalCommander for Terminal<C, I, O>
where
    O: io::Write,
    C: Control,
{
    fn send<T: TerminalRequest>(&mut self, request: T) -> io::Result<()> {
        let mut commander = (&self.control, &mut self.output);
        commander.send(request)
    }
}

impl io::Write for NoIo {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "no io"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "no io"))
    }
}

impl io::Read for NoIo {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "no io"))
    }
}

impl Control for NoIo {
    fn enable_raw_mode(&self) -> std::io::Result<()> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "no io"))
    }
    fn get_size(&self) -> std::io::Result<crate::prelude::Point> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "no io"))
    }
    fn is_tty(&self) -> bool {
        false
    }
    fn is_raw(&self) -> bool {
        false
    }
    fn is_ansi(&self) -> bool {
        false
    }
    fn output(&self) -> &crate::sys::ControlOut {
        panic!("no output")
    }
    fn input(&self) -> &crate::sys::ControlIn {
        panic!("no input")
    }
    fn clone(&self) -> Self
    where
        Self: Sized,
    {
        NoIo
    }
    #[cfg(all(windows, feature = "sys"))]
    fn get_initial_style(&self) -> u16 {
        panic!("no input")
    }
    #[cfg(all(windows, feature = "sys"))]
    fn use_alternate_screen(&self, _alternate: bool) -> std::io::Result<()> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "no io"))
    }
}
