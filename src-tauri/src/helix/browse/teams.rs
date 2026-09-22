//! Teams share Helix auth, rate coordination, body limits and metadata cache.
use super::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_TEAM_MEMBERS: usize = 300;
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamRequest {
    pub name: String,
    pub page: BrowseRequest,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TeamDetails {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub image_url: Option<String>,
    pub members: PagedResult<ChannelSummary>,
    pub member_count: u32,
    pub limited: bool,
}
pub fn normalize_team_name(name: &str) -> Result<String> {
    if name.is_empty()
        || name.len() > 100
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
    {
        return Err(error(ErrorCode::InvalidInput));
    }
    Ok(name.to_ascii_lowercase())
}
impl<A: TwitchApi + 'static> HelixClient<A> {
    pub async fn browse_team(
        &self,
        request: TeamRequest,
        cancel: &CancellationToken,
    ) -> Result<TeamDetails> {
        let name = normalize_team_name(&request.name)?;
        if request.page.cursor.is_some() {
            return Err(error(ErrorCode::InvalidInput));
        }
        let mut session = self.browse_session(&request.page).await?;
        let source = self
            .get_bound::<Team>(
                "teams",
                vec![("name".into(), name.clone())],
                false,
                CacheClass::Metadata,
                request.page.policy(),
                cancel,
                &mut session,
            )
            .await?;
        check_session(&session)?;
        if source.value.data.len() > 1 {
            return Err(error(ErrorCode::InvalidResponse));
        }
        let team = source
            .value
            .data
            .into_iter()
            .next()
            .ok_or_else(|| error(ErrorCode::NotFound))?;
        if !team.team_name.eq_ignore_ascii_case(&name)
            || crate::config::validate_broadcaster_id(&team.id).is_err()
            || team.team_display_name.len() > 256
            || team.info.len() > 16384
            || team.users.len() > 10000
        {
            return Err(error(ErrorCode::InvalidResponse));
        }
        let mut members = team.users;
        for member in &members {
            if crate::config::validate_broadcaster_id(&member.user_id).is_err()
                || crate::streamlink::playback::normalize_login(&member.user_login).is_err()
                || member.user_name.len() > 256
            {
                return Err(error(ErrorCode::InvalidResponse));
            }
        }
        // Twitch does not promise an ordering. Sort deterministically before
        // deduplicating IDs, so repeated members cannot change navigation identity.
        members.sort_by(|a, b| {
            a.user_login
                .to_ascii_lowercase()
                .cmp(&b.user_login.to_ascii_lowercase())
                .then(a.user_id.cmp(&b.user_id))
                .then(a.user_name.cmp(&b.user_name))
        });
        let mut seen = std::collections::HashSet::new();
        members.retain(|m| seen.insert(m.user_id.clone()));
        let member_count = members.len() as u32;
        let limited = members.len() > MAX_TEAM_MEMBERS;
        members.truncate(MAX_TEAM_MEMBERS);
        Ok(TeamDetails {
            id: team.id,
            name,
            display_name: team.team_display_name,
            // Description is text, never HTML or markdown interpreted by the UI.
            description: team.info,
            image_url: image_url(&team.thumbnail_url, 144, 144),
            member_count,
            limited,
            members: PagedResult {
                items: members
                    .into_iter()
                    .map(|m| ChannelSummary {
                        broadcaster_id: m.user_id,
                        login: m.user_login,
                        display_name: m.user_name,
                        image_url: None,
                        followed_at: None,
                        live_state: LiveState::Unknown,
                        title: None,
                        category_name: None,
                        language: None,
                    })
                    .collect(),
                cursor: None,
                freshness: freshness(source.freshness),
                age_seconds: age(source.age.as_secs()),
                warnings: vec![],
            },
        })
    }
}
