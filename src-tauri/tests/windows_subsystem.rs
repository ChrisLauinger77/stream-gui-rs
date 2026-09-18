#![cfg(windows)]

use std::{fs, path::Path, process::Command};

#[test]
fn packaged_entry_point_is_gui_and_development_keeps_its_console() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("entry.rs");
    // Compile the actual entry point with only the Tauri startup call stubbed.
    // This exercises rustc/linker output without a WebView or real credentials.
    let entry =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs")).unwrap();
    fs::write(
        &source,
        format!(
            "{entry}\n{}",
            r#"mod stream_gui_rs {
                pub mod desktop {
                    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
                        if std::env::var_os("STREAM_GUI_RS_TEST_PANIC").is_some() {
                            panic!("synthetic startup panic");
                        }
                        Err("synthetic startup failure".into())
                    }
                }
            }"#
        ),
    )
    .unwrap();
    for (debug, packaged, expected) in [
        (true, false, 3), // Development: Windows CUI.
        (true, true, 2),  // Packaged debug: Windows GUI.
        (false, false, 2),
        (false, true, 2),
    ] {
        let executable = directory.path().join(format!("app-{debug}-{packaged}.exe"));
        let mut compiler =
            Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
        compiler
            .args(["--edition=2024", "--crate-name", "entry_point", "-C"])
            .arg(format!(
                "debug-assertions={}",
                if debug { "yes" } else { "no" }
            ))
            .arg(&source)
            .arg("-o")
            .arg(&executable);
        if packaged {
            compiler.args(["--cfg", r#"feature="custom-protocol""#]);
        }
        let result = compiler.output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let bytes = fs::read(&executable).unwrap();
        assert_eq!(&bytes[..2], b"MZ");
        let pe = u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
        assert_eq!(&bytes[pe..pe + 4], b"PE\0\0");
        // Subsystem is 68 bytes into both PE32 and PE32+ optional headers.
        let offset = pe + 4 + 20 + 68;
        let subsystem = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        assert_eq!(subsystem, expected, "debug={debug}, packaged={packaged}");

        // A GUI subsystem must not replace errors with success or discard
        // explicit diagnostic pipes. The default panic hook remains installed.
        for panic in [false, true] {
            let mut command = Command::new(&executable);
            command.env_remove("STREAM_GUI_RS_TEST_PANIC");
            if panic {
                command.env("STREAM_GUI_RS_TEST_PANIC", "1");
            }
            let result = command.output().unwrap();
            assert_eq!(result.status.code(), Some(if panic { 101 } else { 1 }));
            let message = if panic {
                "synthetic startup panic"
            } else {
                "Could not initialize Stream GUI RS: synthetic startup failure"
            };
            assert!(String::from_utf8_lossy(&result.stderr).contains(message));
        }
    }
}
