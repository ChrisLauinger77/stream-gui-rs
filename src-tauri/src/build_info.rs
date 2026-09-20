//! Public build metadata, embedded at compile time; never resolves Git at runtime.
pub const NAME: &str = "Stream GUI RS";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const COMMIT: &str = env!("STREAM_GUI_RS_BUILD_COMMIT");
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

#[cfg(test)]
mod commit;

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub commit: String,
    pub repository: String,
}
pub fn snapshot() -> AppInfo {
    AppInfo {
        name: NAME.into(),
        version: VERSION.into(),
        commit: COMMIT.into(),
        repository: REPOSITORY.into(),
    }
}
