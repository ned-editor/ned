//! Dummy facade that always fails - no sys
use crate::sys::*;
use crate::Terminal;
use crate::{input::DefaultInput, output::DefaultOutput};
use std::io;

pub type DefaultControl = NoSysControl;
pub type ControlOut = io::Stdout;
pub type ControlIn = io::Stdin;

pub(crate) fn terminal() -> io::Result<Terminal<DefaultControl, DefaultInput, DefaultOutput>> {
    Ok(Terminal::new(
        NoSysControl::default(),
        DefaultInput,
        DefaultOutput::default(),
    ))
}

pub fn get_tty() -> io::Result<DefaultControl> {
    Ok(NoSysControl::default())
}

/// Control in the absence of system/os specific features
#[derive(Debug)]
pub struct NoSysControl {
    input: io::Stdin,
    output: io::Stdout,
    pub size: Option<Point>,
    pub is_raw: bool,
    pub is_ansi: bool,
    pub is_tty: bool,
}

impl Default for NoSysControl {
    fn default() -> Self {
        Self {
            input: io::stdin(),
            output: io::stdout(),
            size: get_terminal_size_no_sys(),
            is_ansi: true,
            is_raw: false,
            is_tty: false,
        }
    }
}

/// Implement what can be done without system dependencies
impl Control for NoSysControl {
    #[allow(deprecated)]
    fn enable_raw_mode(&self) -> std::io::Result<()> {
        get_tty()?.enable_raw_mode()
    }
    fn get_size(&self) -> std::io::Result<Point> {
        self.size
            .ok_or_else(|| std::io::Error::other("Size is not available"))
    }
    fn is_raw(&self) -> bool {
        self.is_raw
    }
    fn is_tty(&self) -> bool {
        self.is_tty
    }
    fn is_ansi(&self) -> bool {
        self.is_ansi
    }
    fn input(&self) -> &io::Stdin {
        &self.input
    }
    fn output(&self) -> &io::Stdout {
        &self.output
    }
    fn clone(&self) -> Self
    where
        Self: Sized,
    {
        NoSysControl::from(self)
    }
}

impl From<&NoSysControl> for NoSysControl {
    fn from(value: &NoSysControl) -> NoSysControl {
        Self {
            input: io::stdin(),
            output: io::stdout(),
            size: value.size,
            is_ansi: value.is_ansi,
            is_raw: value.is_raw,
            is_tty: value.is_tty,
        }
    }
}
