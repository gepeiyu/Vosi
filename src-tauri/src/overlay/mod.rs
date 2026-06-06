use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

#[derive(Clone, Serialize)]
#[serde(tag = "phase", rename_all = "lowercase")]
pub enum OverlayState {
    Hidden,
    Recording { level: f32 },
    Processing,
}

#[derive(Clone)]
pub struct OverlayController {
    app: AppHandle,
    enabled: bool,
}

impl OverlayController {
    pub fn new(app: AppHandle, enabled: bool) -> Self {
        Self { app, enabled }
    }

    pub fn emit(&self, state: OverlayState) {
        if !self.enabled {
            return;
        }
        if let Some(win) = self.app.get_webview_window("overlay") {
            if !matches!(state, OverlayState::Hidden) {
                let _ = win.show();
                self.position_bottom_center(&win);
            } else {
                let _ = win.hide();
            }
            let _ = win.emit("overlay-state", &state);
        }
    }

    fn position_bottom_center<R: Runtime>(&self, win: &tauri::WebviewWindow<R>) {
        if let Ok(monitor) = win.current_monitor() {
            if let Some(m) = monitor {
                let size = m.size();
                let x = (size.width as i32 - 280) / 2;
                let y = size.height as i32 - 56 - 48;
                let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
            }
        }
    }
}
