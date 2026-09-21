//! The only supported native chat client. No credentials, shell or configurable argv.
use crate::{
    domain::{AppError, ErrorCode, Result},
    streamlink::{discovery::SearchLocations, validate_executable},
};
use std::{
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
};

pub fn resolve(path: Option<&str>, locations: &SearchLocations) -> Result<std::path::PathBuf> {
    let found = match path {
        Some(path)
            if path.len() <= 4096
                && !path.chars().any(char::is_control)
                && Path::new(path).is_absolute() =>
        {
            validate_executable(Path::new(path)).ok()
        }
        Some(_) => None,
        None => locations.find("chatterino"),
    };
    found.ok_or_else(|| AppError::new(ErrorCode::ChatterinoNotFound, "Chatterino was not found. Install it or set its executable path; browser chat remains available."))
}
fn arguments(login: &str) -> Result<[String; 2]> {
    crate::streamlink::playback::channel_url(login)?;
    // Upstream's typed --channels layout does not overwrite the saved layout.
    // Direct invocation may create a new independent window; no reuse promise.
    Ok([
        "--channels".into(),
        format!("t:{}", login.to_ascii_lowercase()),
    ])
}
fn launch_error() -> AppError {
    AppError::new(
        ErrorCode::ChatLaunch,
        "Could not start Chatterino. Check its installation or use browser chat.",
    )
}
fn command(path: &Path, login: &str) -> Result<Command> {
    let mut command = Command::new(path);
    command
        .args(arguments(login)?)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // Only desktop/session essentials pass to the independent application. No
    // inherited token/proxy/loader variables, auth state, files or custom environment.
    command.env_clear();
    for key in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SystemRoot",
        "WINDIR",
        "APPDATA",
        "LOCALAPPDATA",
        "TEMP",
        "TMP",
        "TMPDIR",
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XAUTHORITY",
        "DBUS_SESSION_BUS_ADDRESS",
        "XDG_RUNTIME_DIR",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_DIRS",
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_TYPE",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    #[cfg(unix)]
    {
        use std::{
            ffi::CString,
            os::unix::{ffi::OsStrExt, process::CommandExt},
        };
        let program = CString::new(path.as_os_str().as_bytes()).map_err(|_| launch_error())?;
        let args: Vec<CString> = command
            .get_args()
            .map(|arg| CString::new(arg.as_bytes()).map_err(|_| launch_error()))
            .collect::<Result<_>>()?;
        let environment: Vec<CString> = command
            .get_envs()
            .filter_map(|(key, value)| {
                value.map(|value| {
                    let mut entry = key.as_bytes().to_vec();
                    entry.push(b'=');
                    entry.extend_from_slice(value.as_bytes());
                    CString::new(entry).map_err(|_| launch_error())
                })
            })
            .collect::<Result<_>>()?;
        if environment.len() >= 64 || args.len() != 2 {
            return Err(launch_error());
        }
        // SAFETY: captures own stable CString buffers prepared before fork. The
        // callback only fills stack arrays and calls async-signal-safe setsid /
        // execve. Direct execve intentionally prevents execvp's ENOEXEC fallback
        // from interpreting a malformed executable as a shell script. The std
        // spawn error pipe still reports a failed pre_exec and reaps that child.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                let argv = [
                    program.as_ptr(),
                    args[0].as_ptr(),
                    args[1].as_ptr(),
                    std::ptr::null(),
                ];
                let mut envp = [std::ptr::null(); 64];
                for (slot, value) in envp.iter_mut().zip(&environment) {
                    *slot = value.as_ptr();
                }
                libc::execve(program.as_ptr(), argv.as_ptr(), envp.as_ptr());
                Err(std::io::Error::last_os_error())
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Deliberately not suspended or assigned to Streamlink's kill-on-close job.
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    Ok(command)
}
#[derive(Clone)]
pub struct Chatterino {
    children: Arc<tokio::sync::Semaphore>,
}
impl Default for Chatterino {
    fn default() -> Self {
        Self {
            children: Arc::new(tokio::sync::Semaphore::new(16)),
        }
    }
}
impl Chatterino {
    pub fn open(&self, path: Option<&str>, login: &str) -> Result<()> {
        let executable = resolve(path, &SearchLocations::system())?;
        let mut command = command(&executable, login)?;
        let permit = self.children.clone().try_acquire_owned().map_err(|_| {
            AppError::new(
                ErrorCode::ChatterinoCapacity,
                "Sixteen Chatterino launches are still running. Close one or use browser chat.",
            )
        })?;
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        // One bounded waiter per child owns wait/reaping even if the caller leaves.
        // These independent apps are never killed or awaited by playback/Quit.
        // At parent process exit the OS adopts surviving Unix children; Windows
        // retains independent processes after these non-owning handles close.
        std::thread::Builder::new()
            .name("chatterino-reaper".into())
            .spawn(move || {
                let _permit = permit;
                match command.spawn() {
                    Ok(mut child) => {
                        let _ = send.send(Ok(()));
                        let _ = child.wait();
                    }
                    Err(_) => {
                        let _ = send.send(Err(launch_error()));
                    }
                }
            })
            .map_err(|_| launch_error())?;
        receive.recv().unwrap_or_else(|_| Err(launch_error()))
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn active_launchers(&self) -> usize {
        16 - self.children.available_permits()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_arguments_paths_and_environment_do_not_forward_credentials() {
        assert_eq!(
            arguments("Example_1").unwrap(),
            ["--channels", "t:example_1"]
        );
        for login in [
            "",
            "-flag",
            "name;other",
            "a b",
            "name/evil",
            "oauth:synthetic",
            "ユーザー",
            "a\narg",
        ] {
            assert!(arguments(login).is_err());
        }
        let cmd = command(Path::new("/synthetic path/播放器"), "example").unwrap();
        assert_eq!(cmd.get_program(), "/synthetic path/播放器");
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            ["--channels", "t:example"]
        );
        assert!(
            cmd.get_envs()
                .all(|(key, _)| !key.to_string_lossy().to_uppercase().contains("TOKEN"))
        );
        assert!(resolve(Some("relative"), &SearchLocations::system()).is_err());
        assert!(resolve(Some("/absent/chatterino"), &SearchLocations::system()).is_err());
    }
}
