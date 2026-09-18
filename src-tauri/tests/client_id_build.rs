use std::{
    path::Path,
    process::{Command, Output},
};

// Execute the actual build script without the desktop feature so these guards
// are tested on every CI OS without linking Tauri or contacting Twitch.
#[test]
fn build_script_enforces_distribution_configuration() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory
        .path()
        .join(format!("client-id-build{}", std::env::consts::EXE_SUFFIX));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let compilation = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--crate-name", "client_id_build_script"])
        .arg(root.join("build.rs"))
        .arg("-o")
        .arg(&script)
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );

    let run = |profile: &str, distribution: bool, embedded: Option<&str>| -> Output {
        let mut command = Command::new(&script);
        command
            .current_dir(root)
            .env("PROFILE", profile)
            .env("DEBUG", "true")
            .env("TWITCH_CLIENT_ID", "runtimeOverrideCannotSatisfyBuild")
            .env_remove("TWITCH_CLIENT_ID_BUILD")
            .env_remove("CARGO_FEATURE_CUSTOM_PROTOCOL");
        if distribution {
            command.env("CARGO_FEATURE_CUSTOM_PROTOCOL", "1");
        }
        if let Some(value) = embedded {
            command.env("TWITCH_CLIENT_ID_BUILD", value);
        }
        command.output().unwrap()
    };

    for (profile, distribution) in [
        ("release", false),
        ("release", true),
        ("debug", true),
        ("distribution", false),
    ] {
        let output = run(profile, distribution, None);
        assert!(
            !output.status.success(),
            "unguarded {profile}/{distribution}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("Release/distribution builds require TWITCH_CLIENT_ID_BUILD")
        );
    }
    for (profile, distribution) in [
        ("release", false),
        ("release", true),
        ("debug", true),
        ("debug", false),
    ] {
        let output = run(profile, distribution, Some("syntheticBuildClient123"));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("cargo:rerun-if-env-changed=TWITCH_CLIENT_ID_BUILD"));
        assert!(stdout.contains(
            "cargo:rustc-env=STREAM_GUI_RS_EMBEDDED_TWITCH_CLIENT_ID=syntheticBuildClient123\n"
        ));
    }
    for value in [
        "",
        " ",
        " leading",
        "trailing ",
        "invalid-id",
        "id\ncargo:rustc-env=INJECTED=value",
        "ü",
        &"a".repeat(129),
    ] {
        for (profile, distribution) in [("release", false), ("debug", true), ("debug", false)] {
            let output = run(profile, distribution, Some(value));
            assert!(!output.status.success());
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains("TWITCH_CLIENT_ID_BUILD must contain")
            );
            assert!(!String::from_utf8_lossy(&output.stdout).contains("cargo:rustc-env="));
        }
    }
    // Reusing a dev build after removing the input must explicitly clear the ID.
    let output = run("debug", false, None);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("cargo:rustc-env=STREAM_GUI_RS_EMBEDDED_TWITCH_CLIENT_ID=\n"));
    assert!(!stdout.contains("runtimeOverrideCannotSatisfyBuild"));
}
