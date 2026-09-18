use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowseRequest {
    pub session_id: String,
    pub cursor: Option<String>,
    pub refresh: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntityRequest {
    pub page: BrowseRequest,
    pub id: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchRequest {
    pub page: BrowseRequest,
    pub query: String,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DataFreshness {
    Network,
    Cached,
    Stale,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LiveState {
    Live,
    Offline,
    Unknown,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StreamSummary {
    pub stream_id: String,
    pub broadcaster_id: String,
    pub login: String,
    pub display_name: String,
    pub title: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub preview_url: Option<String>,
    pub viewer_count: u32,
    pub language: Option<String>,
    pub started_at: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CategorySummary {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSummary {
    pub broadcaster_id: String,
    pub login: String,
    pub display_name: String,
    pub image_url: Option<String>,
    pub followed_at: Option<String>,
    pub live_state: LiveState,
    pub title: Option<String>,
    pub category_name: Option<String>,
    pub language: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PagedResult<T> {
    pub items: Vec<T>,
    pub cursor: Option<String>,
    pub freshness: DataFreshness,
    pub age_seconds: u32,
    pub warnings: Vec<crate::domain::ErrorCode>,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChannelDetails {
    pub channel: ChannelSummary,
    pub description: Option<String>,
    pub stream: Option<StreamSummary>,
    pub freshness: DataFreshness,
    pub age_seconds: u32,
    pub warnings: Vec<crate::domain::ErrorCode>,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDetails {
    pub category: CategorySummary,
    pub streams: PagedResult<StreamSummary>,
}
