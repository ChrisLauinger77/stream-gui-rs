//! Native executable used only by the integration suite, never a shell script.
use std::{
    io::{self, Write},
    time::Duration,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let executable = std::env::current_exe().unwrap();
    let name = executable.file_stem().unwrap().to_string_lossy();
    if args == ["--version"] {
        if name.contains("timeout") {
            std::fs::write(
                executable.with_extension("pid"),
                std::process::id().to_string(),
            )
            .unwrap();
            std::thread::sleep(Duration::from_secs(30));
        }
        if name.contains("badversion") {
            println!("python 3.12.0");
            return;
        }
        if name.contains("badexit") {
            std::process::exit(9);
        }
        println!("streamlink 8.6.1");
        return;
    }
    if args == ["--descendant"] {
        std::thread::sleep(Duration::from_secs(30));
        return;
    }
    assert_eq!(&args[..4], ["--no-config", "--loglevel", "info", "--"]);
    let url = &args[4];
    if url.ends_with("/flood") {
        let stderr = std::thread::spawn(|| {
            for n in 0..4000 {
                eprintln!("stderr {n}");
            }
        });
        for n in 0..4000 {
            println!("stdout {n}");
        }
        stderr.join().unwrap();
        println!("{}", "x".repeat(100_000));
        println!("Authorization: Bearer DO_NOT_LEAK");
        println!("last line");
    } else if url.ends_with("/hold") || url.ends_with("/tree") {
        if url.ends_with("/tree") {
            let child = std::process::Command::new(executable)
                .arg("--descendant")
                .spawn()
                .unwrap();
            println!("descendant {}", child.id());
            // The supervisor owns the tree; this parent deliberately doesn't
            // reap the descendant to simulate an external process tree.
            std::mem::forget(child);
        }
        println!("ready");
        io::stdout().flush().unwrap();
        std::thread::sleep(Duration::from_secs(30));
    } else {
        println!("stdout ready");
        eprintln!("stderr ready");
        print!("partial stdout");
        eprint!("partial stderr");
        if url.ends_with("/fail") {
            std::process::exit(7);
        }
    }
}
