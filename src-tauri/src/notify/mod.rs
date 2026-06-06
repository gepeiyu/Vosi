use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

#[derive(Clone)]
pub struct Notifier {
    app: AppHandle,
}

impl Notifier {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    pub fn error(&self, body: &str) {
        let _ = self
            .app
            .notification()
            .builder()
            .title("Vosi")
            .body(body)
            .show();
    }
}
