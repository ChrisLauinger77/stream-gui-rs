use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPermission {
    Unknown,
    NotRequested,
    Granted,
    Denied,
    Unavailable,
    OsManaged,
}

#[derive(Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NotificationTestAction {
    Send,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum DesktopAction {
    Channel {
        id: String,
        auth_session_id: String,
        broadcaster_id: String,
        display_name: String,
    },
    Watching {
        id: String,
    },
}
impl DesktopAction {
    pub fn id(&self) -> &str {
        match self {
            Self::Channel { id, .. } | Self::Watching { id } => id,
        }
    }
}
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DesktopStatus {
    pub monitor: crate::monitor::MonitorStatus,
    pub notification_permission: NotificationPermission,
    pub notification_click_supported: bool,
    pub notification_test_available: bool,
    pub tray_available: bool,
    pub action: Option<DesktopAction>,
}
#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcknowledgeDesktopAction {
    pub id: String,
}
