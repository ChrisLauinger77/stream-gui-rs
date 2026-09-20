//! Application queries: bounded pages, intentional public DTOs, one auth
//! session for every primary request and its metadata enrichment.
mod dto;
use super::*;
pub use dto::*;
use std::collections::HashMap;

const PAGE_SIZE: u8 = 30;

impl BrowseRequest {
    fn pagination(&self) -> Result<Vec<(String, String)>> {
        match &self.cursor {
            Some(cursor) => PageRequest::after(PAGE_SIZE, cursor),
            None => PageRequest::first(PAGE_SIZE),
        }?
        .query(false)
    }
    fn policy(&self) -> CachePolicy {
        if self.refresh {
            CachePolicy::Refresh
        } else {
            CachePolicy::Fresh
        }
    }
}
fn optional(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}
fn age(seconds: u64) -> u32 {
    seconds.min(u32::MAX.into()) as u32
}
// Only known Twitch CDN images, with fixed application sizes. No proxy or
// original-resolution URL supplied by the webview.
fn image_url(value: &str, width: u16, height: u16) -> Option<String> {
    let sized = value
        .replace("{width}", &width.to_string())
        .replace("{height}", &height.to_string())
        .replace("300x300", "70x70")
        .replace("150x150", "70x70")
        .replace("52x72", "144x192");
    let url = url::Url::parse(&sized).ok()?;
    (url.scheme() == "https"
        && url.host_str() == Some("static-cdn.jtvnw.net")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none())
    .then(|| url.to_string())
}
impl From<Stream> for StreamSummary {
    fn from(s: Stream) -> Self {
        Self {
            stream_id: s.id,
            broadcaster_id: s.user_id,
            login: s.user_login,
            display_name: s.user_name,
            title: s.title,
            category_id: optional(s.game_id),
            category_name: optional(s.game_name),
            preview_url: image_url(&s.thumbnail_url, 480, 270),
            viewer_count: s.viewer_count.min(u32::MAX.into()) as u32,
            language: optional(s.language),
            started_at: optional(s.started_at),
        }
    }
}
impl From<Game> for CategorySummary {
    fn from(g: Game) -> Self {
        Self {
            id: g.id,
            name: g.name,
            image_url: image_url(&g.box_art_url, 144, 192),
        }
    }
}
impl From<SearchChannel> for ChannelSummary {
    fn from(c: SearchChannel) -> Self {
        Self {
            broadcaster_id: c.id,
            login: c.broadcaster_login,
            display_name: c.display_name,
            image_url: image_url(&c.thumbnail_url, 70, 70),
            followed_at: None,
            live_state: if c.is_live {
                LiveState::Live
            } else {
                LiveState::Offline
            },
            title: optional(c.title),
            category_name: optional(c.game_name),
            language: optional(c.broadcaster_language),
        }
    }
}
fn page<T, U: From<T>>(source: Cached<Page<T>>) -> PagedResult<U> {
    PagedResult {
        items: source.value.data.into_iter().map(Into::into).collect(),
        cursor: source.value.pagination.cursor.filter(|s| !s.is_empty()),
        freshness: freshness(source.freshness),
        age_seconds: age(source.age.as_secs()),
        warnings: vec![],
    }
}
fn freshness(f: Freshness) -> DataFreshness {
    match f {
        Freshness::Network => DataFreshness::Network,
        Freshness::Cached => DataFreshness::Cached,
        Freshness::Stale => DataFreshness::Stale,
    }
}
fn check_session(session: &Option<RequestSession>) -> Result<()> {
    if session.as_ref().is_none_or(|s| s.cancel.is_cancelled()) {
        Err(error(ErrorCode::Unauthenticated))
    } else {
        Ok(())
    }
}
fn terminal(error: &crate::domain::AppError) -> bool {
    matches!(
        error.code,
        ErrorCode::Unauthenticated
            | ErrorCode::AuthInvalid
            | ErrorCode::Unauthorized
            | ErrorCode::Cancelled
            | ErrorCode::CredentialStore
    )
}
fn merge_freshness<T>(result: &mut PagedResult<T>, f: Freshness, elapsed: Duration) {
    result.age_seconds = result.age_seconds.max(age(elapsed.as_secs()));
    if f == Freshness::Stale {
        result.freshness = DataFreshness::Stale;
    } else if f == Freshness::Cached && result.freshness == DataFreshness::Network {
        result.freshness = DataFreshness::Cached;
    }
}
impl<A: TwitchApi + 'static> HelixClient<A> {
    pub async fn lookup_channel(
        &self,
        request: LookupChannelRequest,
        cancel: &CancellationToken,
    ) -> Result<ChannelIdentity> {
        let login = crate::streamlink::playback::normalize_login(&request.login)?;
        let page = BrowseRequest {
            session_id: request.session_id,
            cursor: None,
            refresh: true,
        };
        let mut session = self.browse_session(&page).await?;
        let source = self
            .get_bound::<User>(
                "users",
                params("login", std::slice::from_ref(&login))?,
                false,
                CacheClass::Metadata,
                CachePolicy::Refresh,
                cancel,
                &mut session,
            )
            .await?;
        check_session(&session)?;
        let user = source
            .value
            .data
            .into_iter()
            .find(|user| user.login.eq_ignore_ascii_case(&login))
            .ok_or_else(|| error(ErrorCode::NotFound))?;
        crate::config::validate_broadcaster_id(&user.id)?;
        Ok(ChannelIdentity {
            broadcaster_id: user.id,
            display_name: user.display_name,
        })
    }

    pub async fn chat_login(
        &self,
        auth_session_id: String,
        broadcaster_id: String,
        cancel: &CancellationToken,
    ) -> Result<String> {
        crate::config::validate_broadcaster_id(&broadcaster_id)?;
        let request = BrowseRequest {
            session_id: auth_session_id,
            cursor: None,
            refresh: true,
        };
        let mut session = self.browse_session(&request).await?;
        let source = self
            .get_bound::<User>(
                "users",
                params("id", std::slice::from_ref(&broadcaster_id))?,
                false,
                CacheClass::Metadata,
                CachePolicy::Refresh,
                cancel,
                &mut session,
            )
            .await?;
        check_session(&session)?;
        let user = source
            .value
            .data
            .into_iter()
            .find(|user| user.id == broadcaster_id)
            .ok_or_else(|| error(ErrorCode::NotFound))?;
        crate::streamlink::playback::channel_url(&user.login)?;
        Ok(user.login)
    }

    /// Revalidate live identity in Rust; the webview supplies only a broadcaster ID.
    pub async fn playback_stream(
        &self,
        auth_session_id: String,
        broadcaster_id: String,
        cancel: &CancellationToken,
    ) -> Result<StreamSummary> {
        let request = BrowseRequest {
            session_id: auth_session_id,
            cursor: None,
            refresh: true,
        };
        let mut session = self.browse_session(&request).await?;
        let mut query = params("user_id", std::slice::from_ref(&broadcaster_id))?;
        query.push(("first".into(), "1".into()));
        let source = self
            .get_bound::<Stream>(
                "streams",
                query,
                false,
                CacheClass::Live,
                CachePolicy::Refresh,
                cancel,
                &mut session,
            )
            .await?;
        check_session(&session)?;
        let stream = source
            .value
            .data
            .into_iter()
            .find(|stream| stream.user_id == broadcaster_id)
            .ok_or_else(|| error(ErrorCode::StreamOffline))?;
        crate::streamlink::playback::channel_url(&stream.user_login)?;
        Ok(stream.into())
    }
    async fn browse_session(&self, request: &BrowseRequest) -> Result<Option<RequestSession>> {
        let id = request
            .session_id
            .parse::<u64>()
            .map_err(|_| error(ErrorCode::InvalidInput))?;
        let lease = self.auth.lease_for_session(id).await?;
        Ok(Some(RequestSession {
            id,
            user_id: lease.user_id,
            cancel: lease.cancel,
            background: false,
        }))
    }
    async fn browse_stream_page(
        &self,
        request: &BrowseRequest,
        endpoint: &str,
        extra: Vec<(String, String)>,
        followed: bool,
        cancel: &CancellationToken,
        session: &mut Option<RequestSession>,
    ) -> Result<PagedResult<StreamSummary>> {
        let mut query = request.pagination()?;
        query.extend(extra);
        let source = self
            .get_bound::<Stream>(
                endpoint,
                query,
                followed,
                CacheClass::Live,
                request.policy(),
                cancel,
                session,
            )
            .await?;
        check_session(session)?;
        Ok(page(source))
    }
    pub async fn browse_followed_streams(
        &self,
        request: BrowseRequest,
        cancel: &CancellationToken,
    ) -> Result<PagedResult<StreamSummary>> {
        let mut session = self.browse_session(&request).await?;
        self.browse_stream_page(
            &request,
            "streams/followed",
            vec![],
            true,
            cancel,
            &mut session,
        )
        .await
    }
    pub async fn browse_streams(
        &self,
        request: StreamBrowseRequest,
        cancel: &CancellationToken,
    ) -> Result<PagedResult<StreamSummary>> {
        let mut session = self.browse_session(&request.page).await?;
        self.discovery_stream_page(&request, None, cancel, &mut session)
            .await
    }
    async fn discovery_stream_page(
        &self,
        request: &StreamBrowseRequest,
        category: Option<&str>,
        cancel: &CancellationToken,
        session: &mut Option<RequestSession>,
    ) -> Result<PagedResult<StreamSummary>> {
        let mut page = request.page.clone();
        if let Some(cursor) = &page.cursor {
            if cursor.len() > 16384 {
                return Err(error(ErrorCode::InvalidInput));
            }
            let cursor: DiscoveryCursor =
                serde_json::from_str(cursor).map_err(|_| error(ErrorCode::InvalidInput))?;
            if cursor.session_id != page.session_id
                || cursor.language != request.language
                || cursor.category.as_deref() != category
            {
                return Err(error(ErrorCode::InvalidInput));
            }
            page.cursor = Some(cursor.provider);
        }
        let mut extra = vec![];
        if let Some(id) = category {
            extra.extend(params("game_id", &[id.to_owned()])?);
        }
        if let Some(language) = request.language {
            extra.push(("language".into(), language.code()));
        }
        let mut result = self
            .browse_stream_page(&page, "streams", extra, false, cancel, session)
            .await?;
        if let Some(provider) = result.cursor.take() {
            // Keep Twitch's cursor opaque, but bind its application envelope to the
            // exact discovery query and original auth session. No extra cursor cache.
            PageRequest::after(PAGE_SIZE, &provider)?;
            result.cursor = Some(
                serde_json::to_string(&DiscoveryCursor {
                    session_id: page.session_id,
                    language: request.language,
                    category: category.map(str::to_owned),
                    provider,
                })
                .map_err(|_| error(ErrorCode::Internal))?,
            );
        }
        Ok(result)
    }
    pub async fn browse_categories(
        &self,
        request: BrowseRequest,
        cancel: &CancellationToken,
    ) -> Result<PagedResult<CategorySummary>> {
        let mut session = self.browse_session(&request).await?;
        let source = self
            .get_bound::<Game>(
                "games/top",
                request.pagination()?,
                false,
                CacheClass::Live,
                request.policy(),
                cancel,
                &mut session,
            )
            .await?;
        check_session(&session)?;
        Ok(page(source))
    }
    pub async fn browse_category(
        &self,
        request: CategoryStreamsRequest,
        cancel: &CancellationToken,
    ) -> Result<CategoryDetails> {
        let mut session = self.browse_session(&request.page.page).await?;
        let game = self
            .get_bound::<Game>(
                "games",
                params("id", std::slice::from_ref(&request.id))?,
                false,
                CacheClass::Categories,
                CachePolicy::Fresh,
                cancel,
                &mut session,
            )
            .await?;
        let category = game
            .value
            .data
            .into_iter()
            .find(|g| g.id == request.id)
            .ok_or_else(|| error(ErrorCode::NotFound))?
            .into();
        let streams = self
            .discovery_stream_page(&request.page, Some(&request.id), cancel, &mut session)
            .await?;
        Ok(CategoryDetails { category, streams })
    }
    pub async fn browse_search_channels(
        &self,
        request: SearchRequest,
        cancel: &CancellationToken,
    ) -> Result<PagedResult<ChannelSummary>> {
        let mut session = self.browse_session(&request.page).await?;
        let mut query = search_query(&request)?;
        query.push(("live_only".into(), "false".into()));
        let source = self
            .get_bound::<SearchChannel>(
                "search/channels",
                query,
                false,
                CacheClass::Live,
                request.page.policy(),
                cancel,
                &mut session,
            )
            .await?;
        check_session(&session)?;
        Ok(page(source))
    }
    pub async fn browse_search_categories(
        &self,
        request: SearchRequest,
        cancel: &CancellationToken,
    ) -> Result<PagedResult<CategorySummary>> {
        let mut session = self.browse_session(&request.page).await?;
        let source = self
            .get_bound::<Game>(
                "search/categories",
                search_query(&request)?,
                false,
                CacheClass::Categories,
                request.page.policy(),
                cancel,
                &mut session,
            )
            .await?;
        check_session(&session)?;
        Ok(page(source))
    }
    pub async fn browse_followed_channels(
        &self,
        request: BrowseRequest,
        cancel: &CancellationToken,
    ) -> Result<PagedResult<ChannelSummary>> {
        let mut session = self.browse_session(&request).await?;
        let source = self
            .get_bound::<FollowedChannel>(
                "channels/followed",
                request.pagination()?,
                true,
                CacheClass::Live,
                request.policy(),
                cancel,
                &mut session,
            )
            .await?;
        let ids: Vec<_> = source
            .value
            .data
            .iter()
            .map(|c| c.broadcaster_id.clone())
            .collect();
        let mut result = PagedResult {
            items: source
                .value
                .data
                .into_iter()
                .map(|c| ChannelSummary {
                    broadcaster_id: c.broadcaster_id,
                    login: c.broadcaster_login,
                    display_name: c.broadcaster_name,
                    image_url: None,
                    followed_at: optional(c.followed_at),
                    live_state: LiveState::Unknown,
                    title: None,
                    category_name: None,
                    language: None,
                })
                .collect::<Vec<_>>(),
            cursor: source.value.pagination.cursor.filter(|s| !s.is_empty()),
            freshness: freshness(source.freshness),
            age_seconds: age(source.age.as_secs()),
            warnings: vec![],
        };
        // A single bounded batch for the current page. No per-channel requests
        // or full follow-tree fetch; identical metadata queries reuse Helix cache.
        if !ids.is_empty() {
            match self
                .get_bound::<User>(
                    "users",
                    params("id", &ids)?,
                    false,
                    CacheClass::Metadata,
                    CachePolicy::Fresh,
                    cancel,
                    &mut session,
                )
                .await
            {
                Ok(users) => {
                    merge_freshness(&mut result, users.freshness, users.age);
                    let users: HashMap<_, _> = users
                        .value
                        .data
                        .into_iter()
                        .map(|u| (u.id.clone(), u))
                        .collect();
                    for channel in &mut result.items {
                        if let Some(user) = users.get(&channel.broadcaster_id) {
                            channel.image_url = image_url(&user.profile_image_url, 70, 70);
                        }
                    }
                }
                Err(e) if terminal(&e) => return Err(e),
                Err(e) => result.warnings.push(e.code),
            }
            // Reuse only positive live matches in the freshly cached first
            // followed-stream page. Its omissions cannot establish offline state.
            // Refresh bypasses this reuse, and TTL/session keys stay Rust-owned.
            let bound = session.as_ref().expect("browse session");
            let key = cache_key(
                bound.id,
                &bound.user_id,
                "streams/followed",
                &[
                    ("first".into(), PAGE_SIZE.to_string()),
                    ("user_id".into(), bound.user_id.clone()),
                ],
            )?;
            let cached = self
                .cache
                .lock()
                .expect("cache mutex poisoned")
                .get(&key, request.policy());
            if let Some(cached) = cached {
                let streams: Page<Stream> = decode_page(&cached.value)?;
                let mut reused = false;
                for channel in &mut result.items {
                    if let Some(stream) = streams
                        .data
                        .iter()
                        .find(|s| s.user_id == channel.broadcaster_id)
                    {
                        channel.live_state = LiveState::Live;
                        channel.title = optional(stream.title.clone());
                        channel.category_name = optional(stream.game_name.clone());
                        channel.language = optional(stream.language.clone());
                        reused = true;
                    }
                }
                if reused {
                    merge_freshness(&mut result, cached.freshness, cached.age);
                }
            }
            let ids: Vec<_> = result
                .items
                .iter()
                .filter(|c| c.live_state == LiveState::Unknown)
                .map(|c| c.broadcaster_id.clone())
                .collect();
            if ids.is_empty() {
                check_session(&session)?;
                return Ok(result);
            }
            let mut query = params("user_id", &ids)?;
            query.push(("first".into(), "100".into()));
            match self
                .get_bound::<Stream>(
                    "streams",
                    query,
                    false,
                    CacheClass::Live,
                    request.policy(),
                    cancel,
                    &mut session,
                )
                .await
            {
                Ok(streams) => {
                    merge_freshness(&mut result, streams.freshness, streams.age);
                    let complete = streams
                        .value
                        .pagination
                        .cursor
                        .as_ref()
                        .is_none_or(|c| c.is_empty());
                    let streams: HashMap<_, _> = streams
                        .value
                        .data
                        .into_iter()
                        .map(|s| (s.user_id.clone(), s))
                        .collect();
                    for channel in result
                        .items
                        .iter_mut()
                        .filter(|c| c.live_state == LiveState::Unknown)
                    {
                        if let Some(stream) = streams.get(&channel.broadcaster_id) {
                            channel.live_state = LiveState::Live;
                            channel.title = optional(stream.title.clone());
                            channel.category_name = optional(stream.game_name.clone());
                            channel.language = optional(stream.language.clone());
                        } else if complete {
                            channel.live_state = LiveState::Offline;
                        }
                    }
                    if !complete {
                        result.warnings.push(ErrorCode::InvalidResponse);
                    }
                }
                Err(e) if terminal(&e) => return Err(e),
                Err(e) => result.warnings.push(e.code),
            }
        }
        check_session(&session)?;
        Ok(result)
    }
    pub async fn browse_channel(
        &self,
        request: EntityRequest,
        cancel: &CancellationToken,
    ) -> Result<ChannelDetails> {
        let mut session = self.browse_session(&request.page).await?;
        let source = self
            .get_bound::<User>(
                "users",
                params("id", std::slice::from_ref(&request.id))?,
                false,
                CacheClass::Metadata,
                request.page.policy(),
                cancel,
                &mut session,
            )
            .await?;
        let user = source
            .value
            .data
            .into_iter()
            .find(|u| u.id == request.id)
            .ok_or_else(|| error(ErrorCode::NotFound))?;
        let channel = ChannelSummary {
            broadcaster_id: user.id.clone(),
            login: user.login,
            display_name: user.display_name,
            image_url: image_url(&user.profile_image_url, 70, 70),
            followed_at: None,
            live_state: LiveState::Unknown,
            title: None,
            category_name: None,
            language: None,
        };
        let mut result = ChannelDetails {
            channel,
            description: optional(user.description),
            stream: None,
            freshness: freshness(source.freshness),
            age_seconds: age(source.age.as_secs()),
            warnings: vec![],
        };
        match self
            .get_bound::<Channel>(
                "channels",
                params("broadcaster_id", std::slice::from_ref(&request.id))?,
                false,
                CacheClass::Live,
                request.page.policy(),
                cancel,
                &mut session,
            )
            .await
        {
            Ok(channels) => {
                if let Some(channel) = channels
                    .value
                    .data
                    .into_iter()
                    .find(|c| c.broadcaster_id == request.id)
                {
                    if channels.freshness == Freshness::Cached {
                        result.freshness = DataFreshness::Cached;
                    }
                    result.channel.title = optional(channel.title);
                    result.channel.category_name = optional(channel.game_name);
                    result.channel.language = optional(channel.broadcaster_language);
                    result.age_seconds = result.age_seconds.max(age(channels.age.as_secs()));
                }
            }
            Err(e) if terminal(&e) => return Err(e),
            Err(e) => result.warnings.push(e.code),
        }
        let mut query = params("user_id", std::slice::from_ref(&request.id))?;
        query.push(("first".into(), "100".into()));
        match self
            .get_bound::<Stream>(
                "streams",
                query,
                false,
                CacheClass::Live,
                request.page.policy(),
                cancel,
                &mut session,
            )
            .await
        {
            Ok(streams) => {
                result.age_seconds = result.age_seconds.max(age(streams.age.as_secs()));
                if streams.freshness == Freshness::Cached {
                    result.freshness = DataFreshness::Cached;
                }
                if let Some(stream) = streams
                    .value
                    .data
                    .into_iter()
                    .find(|s| s.user_id == request.id)
                {
                    result.channel.live_state = LiveState::Live;
                    result.channel.title = optional(stream.title.clone());
                    result.channel.category_name = optional(stream.game_name.clone());
                    result.channel.language = optional(stream.language.clone());
                    result.stream = Some(stream.into());
                } else if streams
                    .value
                    .pagination
                    .cursor
                    .as_ref()
                    .is_none_or(|s| s.is_empty())
                {
                    result.channel.live_state = LiveState::Offline;
                } else {
                    result.warnings.push(ErrorCode::InvalidResponse);
                }
            }
            Err(e) if terminal(&e) => return Err(e),
            Err(e) => result.warnings.push(e.code),
        }
        check_session(&session)?;
        Ok(result)
    }
}
fn search_query(request: &SearchRequest) -> Result<Vec<(String, String)>> {
    let search = request.query.trim();
    if search.is_empty() || search.len() > 100 {
        return Err(error(ErrorCode::InvalidInput));
    }
    let mut query = request.page.pagination()?;
    query.extend(params("query", &[search.into()])?);
    Ok(query)
}
#[cfg(test)]
mod tests;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct DiscoveryCursor {
    session_id: String,
    language: Option<crate::config::StreamLanguage>,
    category: Option<String>,
    provider: String,
}
