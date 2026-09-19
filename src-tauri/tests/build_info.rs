use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn compile_script(directory: &Path) -> PathBuf {
    let script = directory.join(format!("build-info{}", std::env::consts::EXE_SUFFIX));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--crate-name", "build_info_script"])
        .arg(root.join("build.rs"))
        .arg("-o")
        .arg(&script)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    script
}

fn build(script: &Path, manifest: &Path, commit: Option<&str>) -> String {
    let mut command = Command::new(script);
    command
        .current_dir(manifest)
        .env("CARGO_MANIFEST_DIR", manifest)
        .env("PROFILE", "debug")
        .env_remove("STREAM_GUI_RS_COMMIT")
        .env_remove("TWITCH_CLIENT_ID_BUILD")
        .env_remove("CARGO_FEATURE_CUSTOM_PROTOCOL");
    if let Some(commit) = commit {
        command.env("STREAM_GUI_RS_COMMIT", commit);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn build_script_embeds_explicit_commit_and_handles_source_archives() {
    let directory = tempfile::tempdir().unwrap();
    let script = compile_script(directory.path());
    // No .git exists here. Use a synthetic source archive, not the real checkout.
    let manifest = directory.path().join("archive/src-tauri");
    std::fs::create_dir_all(&manifest).unwrap();
    for (input, expected) in [
        (
            Some("ABCDEF0123456789abcdef0123456789abcdef0123"),
            "abcdef0",
        ),
        (Some("a1b2c3d"), "a1b2c3d"),
        (None, "unknown"),
        (Some("a1b2c3d\ncargo:rustc-env=INJECTED=yes"), "unknown"),
    ] {
        let stdout = build(&script, &manifest, input);
        assert!(
            stdout.lines().any(
                |line| line == format!("cargo:rustc-env=STREAM_GUI_RS_BUILD_COMMIT={expected}")
            )
        );
        assert!(stdout.contains("cargo:rerun-if-env-changed=STREAM_GUI_RS_COMMIT\n"));
        assert!(!stdout.contains("INJECTED"));
    }
}

#[test]
fn local_git_metadata_tracks_packed_and_new_loose_refs() {
    let directory = tempfile::tempdir().unwrap();
    let script = compile_script(directory.path());
    let root = directory.path().join("checkout");
    let manifest = root.join("src-tauri");
    std::fs::create_dir_all(&manifest).unwrap();
    let config = directory.path().join("empty-git-config");
    std::fs::write(&config, "").unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&root)
            .env("GIT_CONFIG_GLOBAL", &config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args([
                "-c",
                "user.name=Build Test",
                "-c",
                "user.email=build@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "-q"]);
    git(&["commit", "--allow-empty", "-qm", "first synthetic commit"]);
    git(&["pack-refs", "--all", "--prune"]);
    let first = git(&["rev-parse", "HEAD"]);
    let stdout = build(&script, &manifest, None);
    assert!(stdout.contains(&format!(
        "cargo:rustc-env=STREAM_GUI_RS_BUILD_COMMIT={}\n",
        &first[..7]
    )));
    // Git may return forward slashes inside a Windows path. Compare path
    // components rather than requiring the platform's exact separator spelling.
    let watched_paths: Vec<_> = stdout
        .lines()
        .filter_map(|line| line.strip_prefix("cargo:rerun-if-changed="))
        .map(Path::new)
        .collect();
    for name in ["HEAD", "packed-refs", "refs"] {
        let expected = root.join(".git").join(name);
        assert!(
            watched_paths.contains(&expected.as_path()),
            "missing watch for {} in {watched_paths:?}",
            expected.display()
        );
    }
    let packed = std::fs::read(root.join(".git/packed-refs")).unwrap();
    git(&["commit", "--allow-empty", "-qm", "second synthetic commit"]);
    assert_eq!(
        std::fs::read(root.join(".git/packed-refs")).unwrap(),
        packed
    );
    let second = git(&["rev-parse", "HEAD"]);
    assert_ne!(first, second);
    assert!(build(&script, &manifest, None).contains(&format!(
        "cargo:rustc-env=STREAM_GUI_RS_BUILD_COMMIT={}\n",
        &second[..7]
    )));
    assert!(
        build(
            &script,
            &manifest,
            Some("abcdef0123456789abcdef0123456789abcdef0123")
        )
        .contains("cargo:rustc-env=STREAM_GUI_RS_BUILD_COMMIT=abcdef0\n")
    );
}
