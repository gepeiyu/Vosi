#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicrophoneStatus {
    NotDetermined,
    Restricted,
    Denied,
    Authorized,
}

#[link(name = "vosi_mic", kind = "static")]
extern "C" {
    fn vosi_microphone_status() -> i32;
    fn vosi_microphone_authorized() -> bool;
    fn vosi_request_microphone() -> bool;
    fn vosi_open_privacy_settings(pane: *const std::ffi::c_char) -> bool;
    fn vosi_prompt_microphone_denied() -> bool;
    fn vosi_activate_app() -> bool;
    fn vosi_is_accessibility_trusted() -> bool;
    fn vosi_request_accessibility() -> bool;
    fn vosi_hotkey_set_keycode(code: u16);
    fn vosi_hotkey_start(callback: extern "C" fn(event_type: i32)) -> bool;
    fn vosi_hotkey_stop();
}

pub fn microphone_status() -> MicrophoneStatus {
    match unsafe { vosi_microphone_status() } {
        1 => MicrophoneStatus::Restricted,
        2 => MicrophoneStatus::Denied,
        3 => MicrophoneStatus::Authorized,
        _ => MicrophoneStatus::NotDetermined,
    }
}

pub fn is_microphone_authorized() -> bool {
    unsafe { vosi_microphone_authorized() }
}

pub fn request_microphone_permission() -> bool {
    unsafe { vosi_request_microphone() }
}

pub fn open_privacy_settings(pane: &str) -> Result<(), String> {
    let cstr = std::ffi::CString::new(pane).map_err(|e| e.to_string())?;
    if unsafe { vosi_open_privacy_settings(cstr.as_ptr()) } {
        Ok(())
    } else {
        Err(format!("failed to open system settings pane: {pane}"))
    }
}

pub fn prompt_microphone_denied() -> bool {
    unsafe { vosi_prompt_microphone_denied() }
}

pub fn activate_app() {
    let _ = unsafe { vosi_activate_app() };
}

pub fn is_accessibility_trusted() -> bool {
    unsafe { vosi_is_accessibility_trusted() }
}

pub fn request_accessibility() -> bool {
    unsafe { vosi_request_accessibility() }
}

pub fn hotkey_set_keycode(code: u16) {
    unsafe { vosi_hotkey_set_keycode(code) }
}

pub fn hotkey_start(callback: extern "C" fn(event_type: i32)) -> bool {
    unsafe { vosi_hotkey_start(callback) }
}

pub fn hotkey_stop() {
    unsafe { vosi_hotkey_stop() }
}
