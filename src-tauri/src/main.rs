#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = stream_gui_rs::desktop::run() {
        eprintln!("Could not initialize Stream GUI RS: {error}");
        std::process::exit(1);
    }
}
