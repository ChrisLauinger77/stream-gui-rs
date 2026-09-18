// Tauri enables custom-protocol for installed builds, including debug bundles.
// Keep the console for development, but never allocate one for packaged builds.
#![cfg_attr(
    any(not(debug_assertions), feature = "custom-protocol"),
    windows_subsystem = "windows"
)]

fn main() {
    if let Err(error) = stream_gui_rs::desktop::run() {
        eprintln!("Could not initialize Stream GUI RS: {error}");
        std::process::exit(1);
    }
}
