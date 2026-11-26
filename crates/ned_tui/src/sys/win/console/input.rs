use std::{io, os::windows::prelude::AsHandle};
use windows_sys::Win32::System::Console::ENABLE_VIRTUAL_TERMINAL_INPUT;

pub fn is_ansi(input: impl AsHandle) -> io::Result<bool> {
    let console_mode = super::get_console_mode(input)?;
    Ok(0 != console_mode & ENABLE_VIRTUAL_TERMINAL_INPUT)
}

pub fn enable_mouse(input: impl AsHandle, capture: bool) -> io::Result<()> {
    const ENABLE_MOUSE_MODE: u32 = 0x0010 | 0x0080 | 0x0008;

    let mode = super::get_console_mode(&input)?;
    let mode = if capture {
        mode | ENABLE_MOUSE_MODE
    } else {
        mode & !ENABLE_MOUSE_MODE
    };
    super::set_console_mode(input, mode)?;
    Ok(())
}
