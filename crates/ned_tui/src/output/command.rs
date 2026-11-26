//! Commands used internally by termit, also available for more precise control

use crate::log::trace;
#[cfg(all(windows, feature = "sys"))]
use windows_sys::Win32::System::Console::{COORD, SMALL_RECT};

use crate::output::ansi::CSI;
use crate::{prelude::Point, sys::Control};
use std::fmt::Debug;
use std::io;

/// Who executes terminal commands?
pub trait TerminalCommander {
    /// Sends a terminal request command.
    fn send<T: TerminalRequest>(&mut self, request: T) -> io::Result<()>;
}

impl<C: Control, W: io::Write> TerminalCommander for (C, W) {
    fn send<T: TerminalRequest>(&mut self, request: T) -> io::Result<()> {
        trace!("executing terminal request {request:?}");

        let (ref control, ref mut write) = *self;

        let mut ansi_tried = false;

        if control.is_ansi() {
            if request.apply(&mut *write)? {
                // happy ansi path
                return Ok(());
            }
            // it is probably not an ansi command
            ansi_tried = true
        }

        // we're going to run ioctl/winapi call which is immediate, flush first
        write.flush()?;

        // Both calls are attempts with side effects, so the short circuit is
        // deliberate: `apply` only runs when `control` did not handle it.
        if request.control(control)? || (!ansi_tried && request.apply(write)?) {
            Ok(())
        } else {
            Err(io::Error::other(format!("invalid request {request:?}")))
        }
    }
}

/// Request is a nicer word for command
pub trait TerminalRequest: Debug {
    /// The command is sent via output byte stream, ANSI TTY way.
    fn apply(&self, _write: impl io::Write) -> io::Result<bool> {
        Ok(false)
    }
    /// The commander will call this method if the control is not explicitly an ANSI terminal.
    /// This will be the case on legacy windows and in the absence of sys.
    /// The command should make use of the control handles to impart the desired action.
    /// If it doesn't have such a solution, it shall return `None` and let the
    /// commander call `apply` instead. We split the methods like this
    /// because control actions are immediate while ANSI sequences are buffered.
    /// We flush the output when control is to be invoked.
    /// TODO: return `Option<closure>` instead to make this more efficient on non-sys.
    fn control(&self, _control: &impl Control) -> io::Result<bool> {
        Ok(false)
    }
}

/// A command that switches the terminal to 'raw' mode,
/// causing it to send unbuffered/unprocessed input among others.
///
/// # Notes
/// * Commands must be sent for execution otherwise they do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnterRawMode;
impl TerminalRequest for EnterRawMode {
    fn control(&self, _control: &impl Control) -> io::Result<bool> {
        _control.enable_raw_mode().map(|()| true)
    }
}

/// A command that forces buffered output to flush.
///
/// # Notes
/// * Commands must be sent for execution otherwise they do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flush;
impl TerminalRequest for Flush {
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        write.flush().map(|()| true)
    }
}

/// A command that moves the terminal cursor relative to the current position (column, row).
///
/// # Notes
/// * 0,0 is a no-op
/// * Commands must be sent for execution otherwise they do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveCursorBy(pub i16, pub i16);
impl TerminalRequest for MoveCursorBy {
    /// Moves the cursor n (default 1) cells in the given direction. If the cursor is already at the edge of the screen, this has no effect.
    /// CSI n A CUU Cursor Up
    /// CSI n B CUD Cursor Down
    /// CSI n C CUF Cursor Forward
    /// CSI n D CUB Cursor Back
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        let Self(x, y) = *self;
        let mut dimension = |x, p, n| {
            if x > 0 {
                write.write_all(format!("{CSI}{x}{p}").as_bytes())
            } else if x < 0 {
                let x = i16::abs(x);
                write.write_all(format!("{CSI}{x}{n}").as_bytes())
            } else {
                Ok(())
            }
        };

        dimension(x, 'C', 'D')?;
        dimension(y, 'B', 'A')?;
        Ok(true)
    }

    fn control(&self, _control: &impl Control) -> io::Result<bool> {
        #[cfg(all(windows, feature = "sys"))]
        return {
            let Self(x, y) = *self;

            let info =
                crate::sys::console::output::get_console_screen_buffer_info(_control.output())?;
            let COORD {
                X: curr_x,
                Y: curr_y,
            } = info.dwCursorPosition;
            let COORD {
                X: width,
                Y: height,
            } = info.dwSize;
            let SMALL_RECT {
                Left: left,
                Top: top,
                Right: right,
                Bottom: bottom,
            } = info.srWindow;

            let x = curr_x
                .saturating_add(x)
                .clamp(left, width.saturating_sub(right).saturating_sub(1))
                as u16;
            let y = curr_y
                .saturating_add(y)
                .clamp(top, height.saturating_sub(bottom).saturating_sub(1))
                as u16;
            crate::sys::console::output::set_console_cursot_position(_control.output(), x, y)?;
            Ok(true)
        };
        #[allow(unreachable_code)]
        Ok(false)
    }
}

/// A command that moves the terminal cursor to the given position (column, row).
///
/// # Notes
/// * Top left cell is represented as `0,0`.
/// * Commands must be sent for execution otherwise they do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveCursorTo(pub Point);
impl TerminalRequest for MoveCursorTo {
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        let &Self(Point { mut col, mut row }) = self;
        col += 1;
        row += 1;
        write
            .write_all(format!("{CSI}{row};{col}H").as_bytes())
            .map(|()| true)
    }

    fn control(&self, _control: &impl Control) -> io::Result<bool> {
        #[cfg(all(windows, feature = "sys"))]
        return crate::sys::console::output::set_console_cursot_position(
            _control.output(),
            self.0.col,
            self.0.row,
        )
        .map(|()| true);
        #[allow(unreachable_code)]
        Ok(false)
    }
}
/// Reset terminal style to the initial
///
/// # Notes
/// * Commands must be sent for execution otherwise they do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetStyles;
impl TerminalRequest for ResetStyles {
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        write
            .write_all(format!("{CSI}0m").as_bytes())
            .map(|()| true)
    }

    fn control(&self, _control: &impl Control) -> io::Result<bool> {
        #[cfg(all(windows, feature = "sys"))]
        return crate::sys::console::output::set_console_text_attribute(
            _control.output(),
            _control.get_initial_style(),
        )
        .map(|()| true);
        #[allow(unreachable_code)]
        Ok(false)
    }
}
/// Clear the screen currently in view
///
/// # Notes
/// * Commands must be sent for execution otherwise they do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearScreen;
impl TerminalRequest for ClearScreen {
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        write
            .write_all(format!("{CSI}2J").as_bytes())
            .map(|()| true)
    }

    fn control(&self, _control: &impl Control) -> io::Result<bool> {
        #[cfg(all(windows, feature = "sys"))]
        return {
            let info =
                crate::sys::console::output::get_console_screen_buffer_info(_control.output())?;
            //let pos = info.dwCursorPosition;
            let size = info.dwSize;
            let style = info.wAttributes;
            let start_location = COORD { X: 0, Y: 0 };
            let cells_to_write = size.X as u32 * size.Y as u32;
            crate::sys::console::output::fill_with_character(
                _control.output(),
                start_location,
                cells_to_write,
                ' ',
            )?;
            crate::sys::console::output::fill_with_attribute(
                _control.output(),
                start_location,
                cells_to_write,
                style,
            )?;
            Ok(true)
        };
        #[allow(unreachable_code)]
        Ok(false)
    }
}

/// Switch to an alternate screen or back.
///
/// # Notes
/// * Commands must be sent for execution otherwise they do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseAlternateScreen(pub bool);

impl TerminalRequest for UseAlternateScreen {
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        let act = if self.0 { 'h' } else { 'l' };
        write
            .write_all(format!("{CSI}?1049{act}").as_bytes())
            .map(|()| true)
    }
    fn control(&self, _control: &impl Control) -> io::Result<bool> {
        #[cfg(all(windows, feature = "sys"))]
        return _control.use_alternate_screen(self.0).map(|()| true);
        #[allow(unreachable_code)]
        Ok(false)
    }
}
/// Show or hide the cursor pointer/block/underline
///
/// # Notes
/// * Commands must be sent for execution otherwise they do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowCursor(pub bool);

impl TerminalRequest for ShowCursor {
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        let act = if self.0 { 'h' } else { 'l' };
        write
            .write_all(format!("{CSI}?25{act}").as_bytes())
            .map(|()| true)
    }
    fn control(&self, _control: &impl Control) -> io::Result<bool> {
        #[cfg(all(windows, feature = "sys"))]
        return crate::sys::console::output::show_cursor(_control.output(), self.0).map(|()| true);
        #[allow(unreachable_code)]
        Ok(false)
    }
}
/// Enable or disable line wrap at the end of the visible line
///
/// # Notes
/// * Commands must be sent for execution otherwise they do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrapLines(pub bool);

impl TerminalRequest for WrapLines {
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        let act = if self.0 { 'h' } else { 'l' };
        write
            .write_all(format!("{CSI}?7{act}").as_bytes())
            .map(|()| true)
    }
    fn control(&self, _control: &impl Control) -> io::Result<bool> {
        #[cfg(all(windows, feature = "sys"))]
        return crate::sys::console::output::wrap_lines(_control.output(), self.0).map(|()| true);
        #[allow(unreachable_code)]
        Ok(false)
    }
}

/// Make the terminal sen (or not) the mouse events
///
/// # Notes
/// * Commands must be sent for execution otherwise they do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureMouse(pub bool);

impl TerminalRequest for CaptureMouse {
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        let act = if self.0 { 'h' } else { 'l' };
        let mut modes = [
            1000, // Normal tracking: Send mouse X & Y on button press and release
            1002, // Button-event tracking: Report button motion events (dragging)
            1003, // Any-event tracking: Report all motion events
            1015, // RXVT mouse mode: Allows mouse coordinates of >223
            1006, // SGR mouse mode: Allows mouse coordinates of >223, preferred over RXVT mode
        ];

        if !self.0 {
            modes.reverse();
        }

        for mode in modes {
            write.write_all(format!("{CSI}?{mode}{act}").as_bytes())?
        }
        Ok(true)
    }
    fn control(&self, _control: &impl Control) -> io::Result<bool> {
        #[cfg(all(windows, feature = "sys"))]
        return crate::sys::console::input::enable_mouse(_control.output(), self.0).map(|()| true);
        #[allow(unreachable_code)]
        Ok(false)
    }
}

/// Make the terminal send (or not) the window focus change events
///
/// # Notes
/// * Commands must be sent for execution otherwise they do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureFocus(pub bool);

impl TerminalRequest for CaptureFocus {
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        let act = if self.0 { 'h' } else { 'l' };
        write.write_all(format!("{CSI}?1004{act}").as_bytes())?;
        Ok(true)
    }
    fn control(&self, _control: &impl Control) -> io::Result<bool> {
        #[cfg(all(windows, feature = "sys"))]
        return Ok(true); // always on on wincon
        #[allow(unreachable_code)]
        Ok(false)
    }
}
