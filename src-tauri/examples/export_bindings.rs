fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/generated.ts");
    std::fs::write(path, stream_gui_rs::domain::typescript_bindings()).unwrap();
}
