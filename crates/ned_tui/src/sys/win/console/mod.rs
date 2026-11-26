pub mod input;
pub mod output;

use std::ffi::OsStr;
use std::io::IsTerminal;
use std::iter::once;
use std::os::windows::io::AsRawHandle;
use std::os::windows::prelude::{AsHandle, FromRawHandle, OsStrExt};
use std::ptr::null_mut;
use std::{fs, io};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::*;

/// ref: https://docs.microsoft.com/en-us/windows/console/getconsolemode
pub fn get_console_mode(handle: impl AsHandle) -> io::Result<u32> {
    let handle = handle.as_handle().as_raw_handle();
    let mut console_mode: u32 = 0;

    super::result(
        "getting console mode",
        0,
        unsafe { GetConsoleMode(handle, &mut console_mode) } as isize,
    )?;

    Ok(console_mode)
}
/// ref: https://docs.microsoft.com/en-us/windows/console/setconsolemode
pub fn set_console_mode(handle: impl AsHandle, console_mode: u32) -> io::Result<()> {
    let handle = handle.as_handle().as_raw_handle();

    super::result(
        "setting console mode",
        0,
        unsafe { SetConsoleMode(handle, console_mode) } as isize,
    )?;

    Ok(())
}

pub fn open_console(output: bool) -> io::Result<fs::File> {
    let name = if output { "CONOUT$" } else { "CONIN$" };
    // ref: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew
    // Using `CreateFileW("CONOUT$", ...)` to retrieve the console handle works correctly even if STDOUT and/or STDERR are redirected
    let console_out_name: Vec<u16> = OsStr::new(name).encode_wide().chain(once(0)).collect();

    let console_handle = unsafe {
        CreateFileW(
            console_out_name.as_ptr(),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };
    super::result("opening console", -1, console_handle as isize)?;

    Ok(unsafe { fs::File::from_raw_handle(console_handle) })
}

pub fn is_tty<FD>(fd: FD) -> Option<FD>
where
    FD: AsHandle,
{
    if fd.as_handle().is_terminal() {
        Some(fd)
    } else {
        None
    }
}
