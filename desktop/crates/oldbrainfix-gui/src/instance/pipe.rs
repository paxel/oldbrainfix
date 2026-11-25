//! The running app's end of its named pipe on Windows (Q12): an
//! instance of the pipe made, and a later start waited for on it; and
//! the check a start makes of the app at the other end. The rest,
//! reading and writing, goes through `std::fs::File`.
//!
//! The pipe's name can be worked out by anyone, so only this user may
//! open it: its security descriptor grants this user's SID alone, and a
//! start hands nothing to a pipe whose server runs as another user, who
//! made it first.

// `CreateNamedPipeW`, `ConnectNamedPipe` and the calls that read and
// grant the user's SID are FFI (#229): unsafe code is allowed in this
// module only, for those calls.
#![allow(unsafe_code)]

use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

use windows_sys::Win32::Foundation::{
    ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetLengthSid, GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
    TOKEN_USER, TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeServerProcessId, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};

/// The bytes the system holds each way.
const BUFFER: u32 = 64 * 1024;

/// The SID of the user `process` runs as, its bytes; none where it
/// cannot be read.
fn user_of(process: HANDLE) -> Option<Vec<u8>> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: `process` is a valid process handle; `token` receives a
    // handle, owned below.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return None;
    }
    // SAFETY: a valid handle, just opened and owned by nothing else.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let mut size = 0u32;
    // SAFETY: asks for the size only, writing nothing but `size`.
    unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            std::ptr::null_mut(),
            0,
            &mut size,
        )
    };
    if (size as usize) < std::mem::size_of::<TOKEN_USER>() {
        return None;
    }
    // Words, so the TOKEN_USER at its start is aligned.
    let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
    // SAFETY: `buffer` holds at least `size` bytes.
    let read = unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        )
    };
    if read == 0 {
        return None;
    }
    // SAFETY: the system wrote a TOKEN_USER at the buffer's start, whose
    // SID lies in the buffer, which outlives these reads.
    unsafe {
        let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
        let sid = user.User.Sid;
        let length = GetLengthSid(sid) as usize;
        Some(std::slice::from_raw_parts(sid.cast::<u8>(), length).to_vec())
    }
}

/// The SID of the user this app runs as.
fn own_user() -> io::Result<Vec<u8>> {
    // SAFETY: the pseudo handle of this process, which needs no closing.
    user_of(unsafe { GetCurrentProcess() }).ok_or_else(io::Error::last_os_error)
}

/// The SID `sid` in its text form, such as `S-1-5-21-…`.
fn sid_text(sid: &[u8]) -> io::Result<String> {
    let mut text: *mut u16 = std::ptr::null_mut();
    // SAFETY: `sid` holds a valid SID; `text` receives a string the
    // system allocated, freed below.
    if unsafe { ConvertSidToStringSidW(sid.as_ptr().cast_mut().cast(), &mut text) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `text` is NUL-ended, read up to its NUL and then freed.
    unsafe {
        let mut length = 0;
        while *text.add(length) != 0 {
            length += 1;
        }
        let said = String::from_utf16_lossy(std::slice::from_raw_parts(text, length));
        LocalFree(text.cast());
        Ok(said)
    }
}

/// A new instance of the pipe `name`, for one later start; with `first`
/// the first, which fails while another program holds the name. Only
/// programs of this user on this computer open it.
pub fn create(name: &str, first: bool) -> io::Result<OwnedHandle> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    // Full access for this user alone, nobody else's inherited.
    let sddl = format!("D:P(A;;GA;;;{})", sid_text(&own_user()?)?);
    let sddl: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: `sddl` is NUL-ended; `descriptor` receives a descriptor the
    // system allocated, freed below.
    let made = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    };
    if made == 0 {
        return Err(io::Error::last_os_error());
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let mut open = PIPE_ACCESS_DUPLEX;
    if first {
        open |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    let mode = PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS;
    // SAFETY: `wide` is a NUL-ended name and `attributes` points to a
    // valid descriptor, both outliving the call.
    let handle = unsafe {
        CreateNamedPipeW(
            wide.as_ptr(),
            open,
            mode,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER,
            BUFFER,
            0,
            &attributes,
        )
    };
    let made = io::Error::last_os_error();
    // SAFETY: the descriptor the system allocated above, used no more.
    unsafe { LocalFree(descriptor) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(made);
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

/// Whether the app at the other end of `pipe`, opened by a start, runs
/// as this user; false where that cannot be told.
pub fn same_user(pipe: &File) -> bool {
    let mut id = 0u32;
    // SAFETY: `pipe` is the client end of a named pipe; `id` receives the
    // server's process id.
    if unsafe { GetNamedPipeServerProcessId(pipe.as_raw_handle(), &mut id) } == 0 {
        return false;
    }
    // SAFETY: opens the process `id` to query it, or gives null.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, id) };
    if process.is_null() {
        return false;
    }
    // SAFETY: a valid handle, just opened and owned by nothing else.
    let process = unsafe { OwnedHandle::from_raw_handle(process) };
    match (user_of(process.as_raw_handle()), own_user()) {
        (Some(theirs), Ok(own)) => theirs == own,
        _ => false,
    }
}
