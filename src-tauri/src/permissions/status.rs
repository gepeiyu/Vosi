use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PermissionState {
    pub id: String,
    pub label: String,
    pub description: String,
    pub granted: bool,
    pub action_label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PermissionsSnapshot {
    pub all_granted: bool,
    pub voice_ready: bool,
    pub permissions: Vec<PermissionState>,
    pub reinstall_tip: Option<String>,
}

impl PermissionsSnapshot {
    pub fn all_granted(voice_ready: bool) -> Self {
        Self {
            all_granted: true,
            voice_ready,
            permissions: Vec::new(),
            reinstall_tip: None,
        }
    }
}
