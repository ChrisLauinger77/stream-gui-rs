fn main() {
    #[cfg(target_os = "linux")]
    {
        let args: Vec<_> = std::env::args().collect();
        stream_gui_rs::desktop::run_background_smoke(
            std::path::Path::new(&args[1]),
            args[2] == "quit",
            std::path::Path::new(&args[3]),
        )
        .unwrap();
    }
}
