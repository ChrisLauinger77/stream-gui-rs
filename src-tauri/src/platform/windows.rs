use std::{
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
};
use tokio::process::Child;
use windows_sys::Win32::{
    Foundation::{ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE},
    System::{Diagnostics::ToolHelp::*, Threading::*},
};

pub(super) fn resume_child(child: &Child) -> io::Result<()> {
    let pid = child
        .id()
        .ok_or_else(|| io::Error::other("missing child PID"))?;
    // Stable Rust/Tokio does not expose the primary thread handle. A process
    // created suspended has not run its primary thread, so locate its sole
    // thread using the documented ToolHelp API. Fail closed if it is ambiguous.
    // SAFETY: handles are checked, entry size is initialized, handles are uniquely owned,
    // and only the thread of our live, job-owned child is resumed.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let snapshot = OwnedHandle::from_raw_handle(snapshot);
        let mut entry: THREADENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
        if Thread32First(snapshot.as_raw_handle(), &mut entry) == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut thread_id = None;
        loop {
            if entry.th32OwnerProcessID == pid && thread_id.replace(entry.th32ThreadID).is_some() {
                return Err(io::Error::other("suspended child has multiple threads"));
            }
            entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
            if Thread32Next(snapshot.as_raw_handle(), &mut entry) == 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(ERROR_NO_MORE_FILES as i32) {
                    return Err(error);
                }
                break;
            }
        }
        let thread_id =
            thread_id.ok_or_else(|| io::Error::other("missing suspended child thread"))?;
        let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, thread_id);
        if thread.is_null() {
            return Err(io::Error::last_os_error());
        }
        let thread = OwnedHandle::from_raw_handle(thread);
        match ResumeThread(thread.as_raw_handle()) {
            1 => Ok(()),
            u32::MAX => Err(io::Error::last_os_error()),
            _ => Err(io::Error::other("unexpected child thread suspend count")),
        }
    }
}
