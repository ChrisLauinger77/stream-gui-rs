use crate::config::Settings;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BackendDiagnostics {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub settings_path: String,
    pub settings: Settings,
    pub auth_configured: bool,
}
