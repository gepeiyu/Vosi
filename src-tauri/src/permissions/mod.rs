mod status;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) mod microphone_macos;

pub use status::{PermissionState, PermissionsSnapshot};

pub use platform::{all_granted, open_settings, run_startup_gate, snapshot};

#[cfg(target_os = "macos")]
pub use microphone_macos::is_accessibility_trusted;

#[cfg(not(target_os = "macos"))]
pub fn is_accessibility_trusted() -> bool {
    true
}

#[cfg(target_os = "macos")]
mod platform {
    pub use super::macos::{all_granted, open_settings, run_startup_gate, snapshot};
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::status::PermissionsSnapshot;
    use crate::audio::capture::AudioCapture;
    use crate::log::Logger;
    use std::sync::Arc;
    use tauri::AppHandle;

    pub fn all_granted() -> bool {
        true
    }

    pub fn snapshot(voice_ready: bool) -> PermissionsSnapshot {
        PermissionsSnapshot::all_granted(voice_ready)
    }

    pub fn open_settings(_permission_id: &str) -> Result<(), String> {
        Ok(())
    }

    pub fn run_startup_gate(_app: &AppHandle, logger: Arc<Logger>) -> bool {
        match AudioCapture::preflight_microphone() {
            Ok(()) => logger.info("microphone permission preflight ok"),
            Err(err) => logger.info(&format!("microphone preflight skipped: {err}")),
        }
        true
    }
}
