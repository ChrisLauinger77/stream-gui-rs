//! Native executable used only by the integration suite, never a shell script.
use std::{
    io::{self, Write},
    time::Duration,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let executable = std::env::current_exe().unwrap();
    let name = executable.file_stem().unwrap().to_string_lossy();
    if args.first().is_some_and(|arg| arg == "--immediate-tree") {
        let child = std::process::Command::new(&executable)
            .arg("--descendant")
            .spawn()
            .unwrap();
        std::fs::write(&args[1], child.id().to_string()).unwrap();
        std::mem::forget(child);
        std::thread::sleep(Duration::from_secs(30));
        return;
    }
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
        if name.contains("unsupported") {
            println!("streamlink 7.6.0");
            return;
        }
        println!("streamlink 8.6.1");
        return;
    }
    if args == ["--descendant"] {
        std::thread::sleep(Duration::from_secs(30));
        return;
    }
    assert_eq!(args[0], "--no-config");
    let boundary = args.iter().position(|arg| arg == "--").unwrap();
    let url = &args[boundary + 1];
    if url.ends_with("/arguments") {
        println!("{}", serde_json::to_string(&args).unwrap());
    } else if url.ends_with("/delayed") {
        std::thread::sleep(Duration::from_millis(300));
        println!("Starting player: advisory only");
        eprintln!("warning: synthetic warning");
        eprintln!("error: synthetic diagnostic, still running");
        let bytes = "split UTF-8: 日本語".as_bytes();
        io::stdout().write_all(&bytes[..14]).unwrap();
        io::stdout().flush().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        io::stdout().write_all(&bytes[14..]).unwrap();
        io::stdout().flush().unwrap();
        std::thread::sleep(Duration::from_millis(200));
    } else if url.ends_with("/latefail") {
        std::thread::sleep(Duration::from_millis(3200));
        std::process::exit(7);
    } else if url.ends_with("/flood") {
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
    } else if url.ends_with("/hold") || url.ends_with("/holdb") || url.ends_with("/tree") {
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
