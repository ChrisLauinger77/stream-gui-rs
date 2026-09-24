//! Wire models follow Helix field names. IDs are opaque strings, and stream IDs
//! are deliberately independent of user/broadcaster IDs. Unknown fields are
//! ignored for forward compatibility; deprecated view counts/email are omitted.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct User {
    pub id: String,
    pub login: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub user_type: String,
    pub broadcaster_type: String,
    pub description: String,
    pub profile_image_url: String,
    pub offline_image_url: String,
    pub created_at: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Channel {
    pub broadcaster_id: String,
    pub broadcaster_login: String,
    pub broadcaster_name: String,
    pub broadcaster_language: String,
    pub game_id: String,
    pub game_name: String,
    pub title: String,
    pub delay: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub content_classification_labels: Vec<String>,
    #[serde(default)]
    pub is_branded_content: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct FollowedChannel {
    pub broadcaster_id: String,
    pub broadcaster_login: String,
    pub broadcaster_name: String,
    pub followed_at: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Stream {
    pub id: String,
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
    pub game_id: String,
    pub game_name: String,
    #[serde(rename = "type")]
    pub stream_type: String,
    pub title: String,
    pub viewer_count: u64,
    pub started_at: String,
    pub language: String,
    pub thumbnail_url: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub box_art_url: String,
    #[serde(default)]
    pub igdb_id: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SearchChannel {
    pub id: String,
    pub broadcaster_login: String,
    pub display_name: String,
    pub broadcaster_language: String,
    pub game_id: String,
    pub game_name: String,
    pub is_live: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    pub thumbnail_url: String,
    pub title: String,
    pub started_at: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TeamMember {
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Team {
    pub id: String,
    pub team_name: String,
    pub team_display_name: String,
    pub background_image_url: Option<String>,
    pub banner: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub info: String,
    pub thumbnail_url: String,
    pub users: Vec<TeamMember>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChannelTeam {
    pub id: String,
    pub team_name: String,
    pub team_display_name: String,
    pub broadcaster_id: String,
    pub broadcaster_login: String,
    pub broadcaster_name: String,
    pub background_image_url: Option<String>,
    pub banner: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub info: String,
    pub thumbnail_url: String,
}

/// Deliberately small frontend DTO, distinct from a raw /users response.
#[derive(Clone, Debug, Deserialize, Serialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub profile_image_url: Option<String>,
}
impl From<User> for Account {
    fn from(user: User) -> Self {
        let profile_image_url = url::Url::parse(&user.profile_image_url)
            .ok()
            .filter(|url| {
                url.scheme() == "https"
                    && url.host_str() == Some("static-cdn.jtvnw.net")
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.port().is_none()
            })
            .map(|url| url.to_string());
        Self {
            id: user.id,
            login: user.login,
            display_name: user.display_name,
            profile_image_url,
        }
    }
}
