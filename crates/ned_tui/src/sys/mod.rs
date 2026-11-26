//! internal system specific code

pub mod common;
#[cfg_attr(all(unix, feature = "sys"), path = "nix/mod.rs")]
#[cfg_attr(all(windows, feature = "sys"), path = "win/mod.rs")]
mod inner;

pub use inner::*;

use crate::prelude::Point;

/// Essential part of a terminal application - controlling the tty.
pub trait Control {
    /// Enable raw terminal mode to receive ANSI input.
    #[must_use = "Resets on drop"]
    fn enable_raw_mode(&self) -> std::io::Result<()>;
    /// Get terminal window size.
    fn get_size(&self) -> std::io::Result<Point>;
    /// Check if it's a tty.
    fn is_tty(&self) -> bool;
    /// Check if it has been switched to raw mode.
    fn is_raw(&self) -> bool;
    /// Check if the control supports ANSI escapes.
    fn is_ansi(&self) -> bool;
    /// Returns reference to output stream handle.
    fn output(&self) -> &ControlOut;
    /// Returns reference to input stream handle.
    fn input(&self) -> &ControlIn;
    /// Creates a cloned control handle.
    fn clone(&self) -> Self
    where
        Self: Sized;
    /// Returns initial console style attribute on Windows.
    #[cfg(all(windows, feature = "sys"))]
    fn get_initial_style(&self) -> u16;
    /// Toggles alternate screen buffer on Windows.
    #[cfg(all(windows, feature = "sys"))]
    fn use_alternate_screen(&self, alternate: bool) -> std::io::Result<()>;
}

impl<T> Control for &T
where
    T: Control,
    T: ?Sized,
{
    fn enable_raw_mode(&self) -> std::io::Result<()> {
        T::enable_raw_mode(self)
    }
    fn get_size(&self) -> std::io::Result<Point> {
        T::get_size(self)
    }
    fn is_tty(&self) -> bool {
        T::is_tty(self)
    }
    fn is_raw(&self) -> bool {
        T::is_raw(self)
    }
    fn is_ansi(&self) -> bool {
        T::is_ansi(self)
    }
    fn output(&self) -> &ControlOut {
        T::output(self)
    }
    fn input(&self) -> &ControlIn {
        T::input(self)
    }
    fn clone(&self) -> Self
    where
        Self: Sized,
    {
        self
    }
    #[cfg(all(windows, feature = "sys"))]
    fn get_initial_style(&self) -> u16 {
        T::get_initial_style(self)
    }
    #[cfg(all(windows, feature = "sys"))]
    fn use_alternate_screen(&self, alternate: bool) -> std::io::Result<()> {
        T::use_alternate_screen(self, alternate)
    }
}
