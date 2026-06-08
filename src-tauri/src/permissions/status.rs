use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PermissionState {
    pub id: String,
    pub label: String,
    pub description: String,
    pub granted: bool,
    pub action_label: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SetupPhase {
    WaitingPermissions,
    InstallingModels,
    LoadingEngine,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct PermissionsSnapshot {
    pub all_granted: bool,
    pub voice_ready: bool,
    pub setup_phase: SetupPhase,
    pub setup_message: Option<String>,
    pub permissions: Vec<PermissionState>,
    pub reinstall_tip: Option<String>,
}

impl PermissionsSnapshot {
    pub fn all_granted(voice_ready: bool) -> Self {
        Self {
            all_granted: true,
            voice_ready,
            setup_phase: if voice_ready {
                SetupPhase::Ready
            } else {
                SetupPhase::LoadingEngine
            },
            setup_message: None,
            permissions: Vec::new(),
            reinstall_tip: None,
        }
    }
}
