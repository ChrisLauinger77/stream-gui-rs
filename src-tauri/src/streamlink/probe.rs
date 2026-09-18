use crate::{
    domain::{AppError, ErrorCode, Result},
    platform::{ProcessTree, configure_process},
};
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command, time::timeout};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub executable: String,
    pub version: String,
}

pub fn parse_version(output: &str) -> Result<String> {
    for line in output.lines() {
        if let Some(version) = line.trim().strip_prefix("streamlink ") {
            let version = version.trim();
            let numbers = version.split(['.', '-', '+']);
            if !version.is_empty()
                && version.len() <= 80
                && version
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || ".-+".contains(c))
                && numbers.take(3).count() == 3
                && version
                    .split('.')
                    .take(2)
                    .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
                && version
                    .split('.')
                    .nth(2)
                    .is_some_and(|part| part.starts_with(|c: char| c.is_ascii_digit()))
            {
                return Ok(version.to_owned());
            }
        }
    }
    Err(AppError::new(
        ErrorCode::InvalidExecutable,
        "The executable did not return a recognizable 'streamlink X.Y.Z' version.",
    ))
}

pub fn validate_executable(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "The custom Streamlink path must be absolute and point to an executable file.",
        ));
    }
    let metadata = std::fs::metadata(path).map_err(|e| {
        AppError::new(
            ErrorCode::InvalidExecutable,
            format!("Cannot inspect Streamlink at {}: {e}", path.display()),
        )
    })?;
    if !metadata.is_file() {
        return Err(AppError::new(
            ErrorCode::InvalidExecutable,
            "Streamlink path is not a regular file.",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(AppError::new(
                ErrorCode::InvalidExecutable,
                "Streamlink file is not marked executable.",
            ));
        }
    }
    #[cfg(windows)]
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
    {
        return Err(AppError::new(
            ErrorCode::InvalidExecutable,
            "Select a native Streamlink .exe; shell scripts and batch files are not supported.",
        ));
    }
    std::fs::canonicalize(path).map_err(|_| {
        AppError::new(
            ErrorCode::InvalidExecutable,
            "Cannot resolve the Streamlink executable path.",
        )
    })
}

pub fn discover(custom_path: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = custom_path.filter(|p| !p.trim().is_empty()) {
        return validate_executable(Path::new(path));
    }
    discover_on_path(env::var_os("PATH").as_deref()).or_else(|_| {
        super::discovery::SearchLocations::system()
            .find("streamlink")
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::StreamlinkNotFound,
                    "Streamlink was not found. Install it or set a custom executable path.",
                )
            })
    })
}

fn discover_on_path(path: Option<&std::ffi::OsStr>) -> Result<PathBuf> {
    let executable = if cfg!(windows) {
        "streamlink.exe"
    } else {
        "streamlink"
    };
    let mut rejected = false;
    if let Some(path) = path {
        for directory in env::split_paths(path).filter(|p| p.is_absolute()) {
            let candidate = directory.join(executable);
            if candidate.exists() {
                match validate_executable(&candidate) {
                    Ok(path) => return Ok(path),
                    Err(_) => rejected = true,
                }
            }
        }
    }
    Err(AppError::new(
        ErrorCode::NotFound,
        if rejected {
            "Streamlink was found on PATH, but no candidate was an executable file. Set a custom absolute path."
        } else {
            "Streamlink was not found on PATH. Install it or set its absolute path; desktop app PATH may differ from your terminal."
        },
    ))
}

pub async fn probe(custom_path: Option<&str>, limit: Duration) -> Result<ProbeResult> {
    let executable = discover(custom_path)?;
    let mut command = Command::new(&executable);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process(&mut command);
    let mut child = command.spawn().map_err(|e| {
        AppError::new(
            ErrorCode::SpawnFailed,
            format!("Could not start Streamlink: {e}"),
        )
    })?;
    let tree = match ProcessTree::attach(&child) {
        Ok(tree) => tree,
        Err(e) => {
            let _ = child.kill().await;
            return Err(AppError::new(
                ErrorCode::ProcessFailed,
                format!("Could not own Streamlink process: {e}"),
            ));
        }
    };
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    // Drain both pipes even after the capture limit. read_to_end() on an
    // unbounded child output would make the timeout insufficient protection.
    let collect = async {
        let (status, out, err) = tokio::join!(child.wait(), capture(stdout), capture(stderr));
        (status, out, err)
    };
    let result = timeout(limit, collect).await;
    match result {
        Err(_) => {
            let _ = tree.terminate();
            let _ = child.kill().await; // kill also waits/reaps; kill_on_drop is backup
            Err(AppError::new(
                ErrorCode::Timeout,
                "Streamlink --version exceeded the probe timeout.",
            ))
        }
        Ok((status, out, err)) => {
            let status = status.map_err(|_| {
                AppError::new(
                    ErrorCode::ProcessFailed,
                    "Failed to wait for the Streamlink probe.",
                )
            })?;
            if !status.success() {
                return Err(AppError::new(
                    ErrorCode::ProbeFailed,
                    format!(
                        "Streamlink --version exited unsuccessfully (code {:?}). Check the executable and its Python/runtime installation.",
                        status.code()
                    ),
                ));
            }
            let out = out.map_err(|_| {
                AppError::new(
                    ErrorCode::ProbeFailed,
                    "Could not read Streamlink version output.",
                )
            })?;
            let err = err.map_err(|_| {
                AppError::new(
                    ErrorCode::ProbeFailed,
                    "Could not read Streamlink version diagnostics.",
                )
            })?;
            let version = parse_version(&String::from_utf8_lossy(&out))
                .or_else(|_| parse_version(&String::from_utf8_lossy(&err)))?;
            Ok(ProbeResult {
                executable: executable.to_string_lossy().into_owned(),
                version,
            })
        }
    }
}

async fn capture(mut reader: impl tokio::io::AsyncRead + Unpin) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            return Ok(output);
        }
        output.extend_from_slice(&buffer[..count.min(8192 - output.len())]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parsing() {
        for version in ["8.6.1", "7.0.0.dev2+g123", "8.0.0-rc1"] {
            assert_eq!(
                parse_version(&format!("streamlink {version}\r\n")).unwrap(),
                version
            );
        }
        for bad in [
            "python 3.12.0",
            "streamlink ",
            "streamlink nope",
            "streamlink 1.2",
            "streamlink 1.2.x",
            "streamlink 1.2.3 --evil",
        ] {
            assert!(parse_version(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn rejects_relative_missing_directory_and_non_executable() {
        assert!(validate_executable(Path::new("streamlink")).is_err());
        let dir = tempfile::tempdir().unwrap();
        assert!(validate_executable(dir.path()).is_err());
        assert!(validate_executable(&dir.path().join("missing")).is_err());
        let file = dir.path().join("plain.txt");
        std::fs::write(&file, "hello").unwrap();
        assert!(validate_executable(&file).is_err());
        assert_eq!(
            discover_on_path(None).unwrap_err().code,
            ErrorCode::NotFound
        );
        assert!(discover(Some("/not/a/real/streamlink")).is_err());
    }

    #[test]
    fn discovers_native_executable_on_explicit_path() {
        let dir = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) {
            "streamlink.exe"
        } else {
            "streamlink"
        };
        std::fs::copy(std::env::current_exe().unwrap(), dir.path().join(name)).unwrap();
        let path = std::env::join_paths([dir.path()]).unwrap();
        assert_eq!(
            discover_on_path(Some(&path)).unwrap(),
            std::fs::canonicalize(dir.path().join(name)).unwrap()
        );
    }
}
