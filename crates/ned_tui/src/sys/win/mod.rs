pub mod console;

use crate::log::{trace, warn};
use crate::prelude::Point;
use crate::{input::DefaultInput, output::DefaultOutput};
use crate::{sys::Control, Terminal};
use std::os::windows::prelude::AsHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{fs, io};
use windows_sys::Win32::System::Console::*;

pub type DefaultControl = WinCon;
pub type ControlOut = fs::File;
pub type ControlIn = fs::File;

pub(crate) fn terminal() -> io::Result<Terminal<DefaultControl, DefaultInput, DefaultOutput>> {
    let control = get_tty()?;
    Ok(Terminal::new(
        control,
        DefaultInput,
        DefaultOutput::default(),
    ))
}

/// Try to get terminal tty from well known IO handles -  stdin, stdout, CONOUT$
pub fn get_tty() -> io::Result<DefaultControl> {
    let input = None
        .or_else(|| console::is_tty(io::stdin()))
        .and_then(|stdin| stdin.as_handle().try_clone_to_owned().ok())
        .map(fs::File::from)
        .or_else(|| console::is_tty(console::open_console(false).ok()?))
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "no in-tty was found"))?;
    let output = None
        .or_else(|| console::is_tty(io::stdout()))
        .and_then(|stdout| stdout.as_handle().try_clone_to_owned().ok())
        .map(fs::File::from)
        .or_else(|| console::is_tty(console::open_console(true).ok()?))
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "no out-tty was found"))?;

    WinCon::try_from((input, output))
}

pub struct WinCon {
    reset: Arc<Reset>,
}
impl TryFrom<(fs::File, fs::File)> for WinCon {
    type Error = io::Error;

    fn try_from(value: (fs::File, fs::File)) -> Result<Self, Self::Error> {
        let (input, output) = value;
        let alt_output = console::output::create_new_screen().ok();
        let state_in = console::get_console_mode(&input)?;
        let state_out = console::get_console_mode(&output)?;
        let style = console::output::get_console_screen_buffer_info(&output)?.wAttributes;
        let alt_output_active = AtomicBool::default();
        Ok(WinCon {
            reset: Arc::new(Reset {
                input,
                output,
                alt_output,
                alt_output_active,
                style,
                state_in,
                state_out,
            }),
        })
    }
}

///
/// ref: https://learn.microsoft.com/en-us/windows/console/reading-input-buffer-events
/// ref: https://docs.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences#EXAMPLE_OF_ENABLING_VIRTUAL_TERMINAL_PROCESSING @@ https://archive.is/L7wRJ#76%
///
impl Control for WinCon {
    fn enable_raw_mode(&self) -> std::io::Result<()> {
        console::set_console_mode(
            self.input(),
            self.reset.state_in | ENABLE_VIRTUAL_TERMINAL_INPUT,
        )?;
        console::set_console_mode(
            self.output(),
            self.reset.state_in | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        )?;
        Ok(())
    }
    fn get_size(&self) -> std::io::Result<Point> {
        console::output::size_from_handle(self.output())
    }
    fn is_tty(&self) -> bool {
        console::is_tty(&self.input()).is_some()
    }
    fn is_raw(&self) -> bool {
        console::input::is_ansi(self.input()).unwrap_or_default()
    }
    fn is_ansi(&self) -> bool {
        console::output::is_ansi(self.output()).unwrap_or_default()
    }
    fn output(&self) -> &fs::File {
        if let Some(alt_out) = self.reset.alt_output.as_ref() {
            if self.reset.alt_output_active.load(Ordering::Relaxed) {
                return alt_out;
            }
        }

        &self.reset.output
    }
    fn input(&self) -> &fs::File {
        &self.reset.input
    }
    fn clone(&self) -> Self {
        Self {
            reset: self.reset.clone(),
        }
    }
    fn get_initial_style(&self) -> u16 {
        self.reset.style
    }
    fn use_alternate_screen(&self, alternate: bool) -> io::Result<()> {
        if alternate && self.reset.alt_output.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "could not get alternate screen",
            ));
        }

        let was = self
            .reset
            .alt_output_active
            .swap(alternate, Ordering::Relaxed);

        match (was, alternate) {
            (true, false) => {
                // disable
                console::output::switch_to_screen(&self.reset.output)
            }
            (false, true) => {
                // enable
                console::output::switch_to_screen(
                    self.reset
                        .alt_output
                        .as_ref()
                        .expect("checked alt screen presence already"),
                )
            }
            _ => {
                // no change
                Ok(())
            }
        }
    }
}

/// Shared reset object - last one switch the lights off, please
struct Reset {
    style: u16,
    state_in: u32,
    state_out: u32,
    input: fs::File,
    output: fs::File,
    alt_output: Option<fs::File>,
    alt_output_active: AtomicBool,
}
impl Drop for Reset {
    fn drop(&mut self) {
        reset(
            self.state_in,
            self.state_out,
            self.style,
            &self.input,
            &self.output,
            self.alt_output_active.load(Ordering::Relaxed),
        )
    }
}
fn reset(si: u32, so: u32, watttr: u16, i: &fs::File, o: &fs::File, ao: bool) {
    if ao {
        trace!("Reactivating primary screen {o:?}");
        console::output::switch_to_screen(o)
            .unwrap_or_else(|e| warn!("could not activate primary screen - {e}"));
    }
    trace!("Resetting console mode for {i:?} and {o:?}");
    console::set_console_mode(i, si).unwrap_or_else(|e| warn!("could not reset in mode - {e}"));
    console::set_console_mode(o, so).unwrap_or_else(|e| warn!("could not reset out mode - {e}"));
    reset_style(watttr, o).unwrap_or_else(|e| warn!("could not reset output style - {e}"))
}
fn reset_style(wattr: u16, o: &fs::File) -> io::Result<()> {
    console::output::set_console_text_attribute(o, wattr)
}

/// Get the result of a call to Windows API as an [`io::Result`].
#[inline]
pub fn result(
    ctx: impl std::fmt::Display,
    error_value: isize,
    return_value: isize,
) -> io::Result<isize> {
    if return_value == error_value {
        let err = io::Error::last_os_error();
        let err = io::Error::new(err.kind(), format!("Error {ctx} - {err:?}"));
        Err(err)
    } else {
        Ok(return_value)
    }
}
