fn main() {
    for name in [
        "icons/icon.png",
        "icons/icon-idle.png",
        "icons/icon-recording.png",
        "icons/icon-warning.png",
    ] {
        println!("cargo:rerun-if-changed={name}");
    }
    tauri_build::build()
}
