#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    stream_gui_rs::desktop::run();
}
