use crate::prelude::{Point, point};
use crate::sys::result;
use std::ffi::c_void;
use std::fs;
use std::os::windows::prelude::FromRawHandle;
use std::{
    io,
    os::windows::io::{AsHandle, AsRawHandle},
};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE,
};
use windows_sys::Win32::System::Console::*;

pub fn is_ansi(output: impl AsHandle) -> io::Result<bool> {
    let console_mode = super::get_console_mode(output)?;
    Ok(0 != console_mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING)
}
pub fn size_from_handle(output: impl AsHandle) -> io::Result<Point> {
    let info = get_console_screen_buffer_info(output)?;
    Ok(point(
        info.srWindow
            .Right
            .saturating_sub(info.srWindow.Left)
            .saturating_add(1)
            .clamp(0, i16::MAX) as u16,
        info.srWindow
            .Bottom
            .saturating_sub(info.srWindow.Top)
            .saturating_add(1)
            .clamp(0, i16::MAX) as u16,
    ))
}
pub fn get_console_screen_buffer_info(
    output: impl AsHandle,
) -> io::Result<CONSOLE_SCREEN_BUFFER_INFO> {
    let handle = output.as_handle().as_raw_handle();

    let mut info = CONSOLE_SCREEN_BUFFER_INFO {
        dwSize: COORD { X: 0, Y: 0 },
        dwCursorPosition: COORD { X: 0, Y: 0 },
        wAttributes: 0,
        srWindow: SMALL_RECT {
            Left: 0,
            Top: 0,
            Right: 0,
            Bottom: 0,
        },
        dwMaximumWindowSize: COORD { X: 0, Y: 0 },
    };
    result("getting screen buffer info", -1, unsafe {
        GetConsoleScreenBufferInfo(handle, &mut info)
    } as isize)?;
    Ok(info)
}
pub fn set_console_cursot_position(output: impl AsHandle, x: u16, y: u16) -> io::Result<()> {
    let handle = output.as_handle().as_raw_handle();
    let position = COORD {
        X: x as i16,
        Y: y as i16,
    };

    result("getting cursor position", 0, unsafe {
        SetConsoleCursorPosition(handle, position) as isize
    })?;

    Ok(())
}

pub fn set_console_text_attribute(output: impl AsHandle, attributes: u16) -> io::Result<()> {
    let handle = output.as_handle().as_raw_handle();
    result("setting console text attributes", 0, unsafe {
        SetConsoleTextAttribute(handle, attributes) as isize
    })?;

    Ok(())
}

/// Writes a character to the console screen buffer a specified number of times, beginning at the specified coordinates.
/// Returns the number of characters that have been written.
///
/// This wraps
/// [`FillConsoleOutputCharacterA`](https://docs.microsoft.com/en-us/windows/console/fillconsoleoutputcharacter).
pub fn fill_with_character(
    output: impl AsHandle,
    start_location: COORD,
    cells_to_write: u32,
    filling_char: char,
) -> io::Result<u32> {
    let handle = output.as_handle().as_raw_handle();

    let mut chars_written = 0;
    result("filling screen with chars", 0, unsafe {
        // fill the cells in console with blanks
        FillConsoleOutputCharacterA(
            handle,
            filling_char as i8,
            cells_to_write,
            start_location,
            &mut chars_written,
        ) as isize
    })?;

    Ok(chars_written)
}

/// Sets the character attributes for a specified number of character cells, beginning at the specified coordinates in a screen buffer.
/// Returns the number of cells that have been modified.
///
/// This wraps
/// [`FillConsoleOutputAttribute`](https://docs.microsoft.com/en-us/windows/console/fillconsoleoutputattribute).
pub fn fill_with_attribute(
    output: impl AsHandle,
    start_location: COORD,
    cells_to_write: u32,
    dw_attribute: u16,
) -> io::Result<u32> {
    let handle = output.as_handle().as_raw_handle();
    let mut cells_written = 0;
    // Get the position of the current console window
    result("filling screen with style", 0, unsafe {
        FillConsoleOutputAttribute(
            handle,
            dw_attribute,
            cells_to_write,
            start_location,
            &mut cells_written,
        ) as isize
    })?;

    Ok(cells_written)
}

const NULL: *mut c_void = 0 as *mut c_void;
const TRUE: i32 = 1;
const FALSE: i32 = 0;

pub fn switch_to_screen(output: impl AsHandle) -> io::Result<()> {
    let handle = output.as_handle().as_raw_handle();

    result("activating a screen buffer", 0, unsafe {
        SetConsoleActiveScreenBuffer(handle) as isize
    })?;

    Ok(())
}
pub fn create_new_screen() -> io::Result<fs::File> {
    let security_attr: SECURITY_ATTRIBUTES = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: NULL,
        bInheritHandle: TRUE,
    };

    let new_screen_buffer = unsafe {
        CreateConsoleScreenBuffer(
            FILE_GENERIC_READ | FILE_GENERIC_WRITE, // R/W
            FILE_SHARE_READ | FILE_SHARE_WRITE,     // shared
            &security_attr,                         // default security attributes
            CONSOLE_TEXTMODE_BUFFER,                // must be TEXTMODE
            NULL,
        )
    };
    // HANDLE is a pointer since windows-sys 0.52, so the sentinel comparison
    // happens on the address rather than on the handle itself.
    result("creating new screen buffer", -1, new_screen_buffer as isize)?;

    Ok(unsafe { fs::File::from_raw_handle(new_screen_buffer) })
}

pub fn show_cursor(output: impl AsHandle, visible: bool) -> io::Result<()> {
    let handle = output.as_handle().as_raw_handle();

    let cursor_info = CONSOLE_CURSOR_INFO {
        dwSize: 100,
        bVisible: if visible { TRUE } else { FALSE },
    };

    result(
        "setting cursor visibility",
        0,
        unsafe { SetConsoleCursorInfo(handle, &cursor_info) } as isize,
    )?;

    Ok(())
}

pub fn wrap_lines(output: impl AsHandle, wrap: bool) -> io::Result<()> {
    let mode = crate::sys::console::get_console_mode(&output)?;
    let mode = if wrap {
        mode | ENABLE_WRAP_AT_EOL_OUTPUT
    } else {
        mode & !ENABLE_WRAP_AT_EOL_OUTPUT
    };
    crate::sys::console::set_console_mode(output, mode)?;
    Ok(())
}
