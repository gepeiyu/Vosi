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
        let Some(win) = self.app.get_webview_window("overlay") else {
            eprintln!("overlay window not found");
            return;
        };
        match &state {
            OverlayState::Hidden => {
                let _ = win.hide();
            }
            OverlayState::Recording { level } => {
                let _ = win.show();
                // Position once when recording starts, not on every level tick.
                if *level == 0.0 {
                    self.position_bottom_center(&win);
                }
            }
            OverlayState::Processing => {
                let _ = win.show();
                self.position_bottom_center(&win);
            }
        }
        let _ = self.app.emit_to("overlay", "overlay-state", &state);
    }

    fn position_bottom_center<R: Runtime>(&self, win: &tauri::WebviewWindow<R>) {
        const OVERLAY_WIDTH: f64 = 280.0;
        const OVERLAY_HEIGHT: f64 = 56.0;
        const BOTTOM_MARGIN: f64 = 48.0;

        let monitor = self
            .app
            .primary_monitor()
            .ok()
            .flatten()
            .or_else(|| win.current_monitor().ok().flatten());

        let Some(m) = monitor else {
            let _ = win.center();
            return;
        };

        let scale = m.scale_factor();
        let work = m.work_area();
        let win_w = (OVERLAY_WIDTH * scale).round() as i32;
        let win_h = (OVERLAY_HEIGHT * scale).round() as i32;
        let margin = (BOTTOM_MARGIN * scale).round() as i32;

        let x = work.position.x + (work.size.width as i32 - win_w) / 2;
        let y = work.position.y + work.size.height as i32 - win_h - margin;
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    }
}
