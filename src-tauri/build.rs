fn main() {
    for name in [
        "icons/icon.png",
        "icons/icon-idle.png",
        "icons/icon-recording.png",
        "icons/icon-warning.png",
    ] {
        println!("cargo:rerun-if-changed={name}");
    }

    #[cfg(target_os = "macos")]
    {
        if std::env::var_os("MACOSX_DEPLOYMENT_TARGET").is_none() {
            std::env::set_var("MACOSX_DEPLOYMENT_TARGET", "12.0");
        }
        println!("cargo:rerun-if-changed=native/microphone_permission.m");
        println!("cargo:rerun-if-changed=native/hotkey_monitor.m");
        cc::Build::new()
            .file("native/microphone_permission.m")
            .file("native/hotkey_monitor.m")
            .flag("-fobjc-arc")
            .compile("vosi_mic");
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=ApplicationServices");
    }

    tauri_build::build()
}
