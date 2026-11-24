//! The running app's end of its named pipe on Windows (Q12): an
//! instance of the pipe made, and a later start waited for on it. The
//! rest, reading and writing, goes through `std::fs::File`.

// `CreateNamedPipeW` and `ConnectNamedPipe` are FFI (#229): unsafe code
// is allowed in this module only, for those two calls.
#![allow(unsafe_code)]

use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

use windows_sys::Win32::Foundation::{ERROR_PIPE_CONNECTED, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

/// The bytes the system holds each way.
const BUFFER: u32 = 64 * 1024;

/// A new instance of the pipe `name`, for one later start; with `first`
/// the first, which fails while another program holds the name. Only
/// programs on this computer connect.
pub fn create(name: &str, first: bool) -> io::Result<OwnedHandle> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut open = PIPE_ACCESS_DUPLEX;
    if first {
        open |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    let mode = PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS;
    // SAFETY: `wide` is a NUL-ended name that outlives the call; no
    // security attributes are handed in, so the pipe has the default
    // ones, of this user's.
    let handle = unsafe {
        CreateNamedPipeW(
            wide.as_ptr(),
            open,
            mode,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER,
            BUFFER,
            0,
            std::ptr::null(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a valid handle, just made and owned by nothing else.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

/// Waits for a later start to connect to `pipe`; the pipe to read its
/// message from and write the answer to.
pub fn accept(pipe: OwnedHandle) -> io::Result<File> {
    // SAFETY: `pipe` is a valid handle of a pipe instance, opened without
    // overlapped I/O, so no OVERLAPPED is handed in.
    let connected = unsafe { ConnectNamedPipe(pipe.as_raw_handle(), std::ptr::null_mut()) };
    if connected == 0 {
        let e = io::Error::last_os_error();
        // A start that connected before the wait began is connected.
        if e.raw_os_error() != Some(ERROR_PIPE_CONNECTED as i32) {
            return Err(e);
        }
    }
    Ok(File::from(pipe))
}
