//! Platform-specific process ownership, independent of Tauri.
use std::io;
use tokio::process::{Child, Command};
#[cfg(windows)]
mod windows;

pub fn configure_process(command: &mut Command) {
    command.kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    #[cfg(windows)]
    command.creation_flags(
        windows_sys::Win32::System::Threading::CREATE_NO_WINDOW
            | windows_sys::Win32::System::Threading::CREATE_SUSPENDED,
    );
}

/// Owned process group on Unix; kill-on-close job on Windows. Descendants that
/// intentionally detach/break away are outside the Phase 0 ownership boundary.
pub struct ProcessTree {
    #[cfg(unix)]
    group: i32,
    #[cfg(windows)]
    job: isize,
}

impl ProcessTree {
    /// Call only for a child spawned with `configure_process`. On Windows this
    /// assigns the still-suspended child to its job before resuming its thread.
    pub fn attach(child: &Child) -> io::Result<Self> {
        #[cfg(unix)]
        {
            Ok(Self {
                group: child
                    .id()
                    .ok_or_else(|| io::Error::other("missing child PID"))?
                    as i32,
            })
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
            let process = child
                .raw_handle()
                .ok_or_else(|| io::Error::other("missing child handle"))?;
            // SAFETY: all structures are initialized, handles checked, and the
            // successful job handle is owned until Drop.
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job.is_null() {
                    return Err(io::Error::last_os_error());
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                if SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    (&info as *const _) as *const _,
                    std::mem::size_of_val(&info) as u32,
                ) == 0
                {
                    let error = io::Error::last_os_error();
                    CloseHandle(job);
                    return Err(error);
                }
                if AssignProcessToJobObject(job, process as _) == 0 {
                    let error = io::Error::last_os_error();
                    CloseHandle(job);
                    return Err(error);
                }
                let tree = Self { job: job as isize };
                windows::resume_child(child)?;
                Ok(tree)
            }
        }
    }

    pub fn terminate(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            // SAFETY: a positive group ID came from our own isolated child.
            if unsafe { libc::kill(-self.group, libc::SIGKILL) } == -1 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error);
                }
            }
            Ok(())
        }
        #[cfg(windows)]
        {
            // SAFETY: this object owns a valid job handle.
            if unsafe {
                windows_sys::Win32::System::JobObjects::TerminateJobObject(self.job as _, 1)
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }
}

impl Drop for ProcessTree {
    fn drop(&mut self) {
        let _ = self.terminate();
        #[cfg(windows)]
        // SAFETY: close exactly once; no other owner of this job handle exists.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.job as _);
        }
    }
}
