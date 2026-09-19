pub mod browse;
pub mod cache;
pub mod models;
mod monitoring;
pub mod pagination;

use crate::{
    domain::{ErrorCode, Result},
    twitch::{AccessLease, AuthService, HttpTwitchApi, api::TwitchApi},
    twitch_http::{TwitchHttp, error},
};
use cache::{CacheClass, CachePolicy, Cached, Freshness, TwitchCache};
use models::*;
use pagination::{BatchFailure, BatchResult, Page, PageRequest, id_batches};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct StreamFilter {
    pub user_ids: Vec<String>,
    pub user_logins: Vec<String>,
    pub game_ids: Vec<String>,
    pub languages: Vec<String>,
}
pub enum TeamSelector<'a> {
    Id(&'a str),
    Name(&'a str),
}

// One binding for the complete operation, including all retries/batch chunks.
struct RequestSession {
    id: u64,
    user_id: String,
    cancel: CancellationToken,
    background: bool,
}

// One canonical key for network requests and cross-view reuse of fresh pages.
fn cache_key(id: u64, user_id: &str, endpoint: &str, query: &[(String, String)]) -> Result<String> {
    let encoded = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(query)
        .finish();
    if encoded.len() > 7000 {
        return Err(error(ErrorCode::InvalidInput));
    }
    // Public session identity, never credentials, isolates account generations.
    Ok(format!("{id}:{user_id}:{endpoint}?{encoded}"))
}

pub struct HelixClient<A: TwitchApi = HttpTwitchApi> {
    http: TwitchHttp,
    auth: Arc<AuthService<A>>,
    cache: Mutex<TwitchCache>,
}
impl<A: TwitchApi + 'static> HelixClient<A> {
    pub fn new(http: TwitchHttp, auth: Arc<AuthService<A>>) -> Self {
        Self {
            http,
            auth,
            cache: Mutex::default(),
        }
    }
    pub fn clear_cache(&self) {
        self.cache.lock().expect("cache mutex poisoned").clear();
    }
    pub fn invalidate(&self, class: CacheClass) {
        self.cache
            .lock()
            .expect("cache mutex poisoned")
            .invalidate(class);
    }
    pub fn rate_status(&self) -> crate::twitch_http::rate::RateSnapshot {
        self.http.rate.snapshot()
    }

    async fn acquire_lease(
        &self,
        cancel: &CancellationToken,
        session: Option<&RequestSession>,
    ) -> Result<AccessLease> {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(error(ErrorCode::Cancelled)),
            _ = async { match session { Some(session) => session.cancel.cancelled().await, None => std::future::pending().await } } => Err(error(ErrorCode::Unauthenticated)),
            lease = async { match session { Some(session) => self.auth.lease_for_session(session.id).await, None => self.auth.lease().await } } => lease,
        }
    }
    async fn get<T: DeserializeOwned + Serialize>(
        &self,
        endpoint: &str,
        query: Vec<(String, String)>,
        followed: bool,
        class: CacheClass,
        policy: CachePolicy,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<T>>> {
        self.get_bound(endpoint, query, followed, class, policy, cancel, &mut None)
            .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn get_bound<T: DeserializeOwned + Serialize>(
        &self,
        endpoint: &str,
        mut query: Vec<(String, String)>,
        followed: bool,
        class: CacheClass,
        policy: CachePolicy,
        cancel: &CancellationToken,
        session: &mut Option<RequestSession>,
    ) -> Result<Cached<Page<T>>> {
        for attempt in 0..2 {
            let lease = self.acquire_lease(cancel, session.as_ref()).await?;
            session.get_or_insert_with(|| RequestSession {
                id: lease.session_id,
                user_id: lease.user_id.clone(),
                cancel: lease.cancel.clone(),
                background: false,
            });
            if lease.cancel.is_cancelled() {
                return Err(error(ErrorCode::Unauthenticated));
            }
            if followed {
                if !lease
                    .scopes
                    .iter()
                    .any(|scope| scope == "user:read:follows")
                {
                    return Err(error(ErrorCode::Unauthorized));
                }
                query.retain(|(key, _)| key != "user_id");
                query.push(("user_id".into(), lease.user_id.clone()));
            }
            let key = cache_key(lease.session_id, &lease.user_id, endpoint, &query)?;
            if let Some(cached) = self
                .cache
                .lock()
                .expect("cache mutex poisoned")
                .get(&key, policy)
            {
                return Ok(Cached {
                    value: decode_page(&cached.value)?,
                    freshness: cached.freshness,
                    age: cached.age,
                });
            }
            let response = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(error(ErrorCode::Cancelled)),
                _ = lease.cancel.cancelled() => return Err(error(ErrorCode::Unauthenticated)),
                result = self.http.get_with_priority(endpoint, &query, &lease.client_id, &lease.token, cancel, session.as_ref().is_some_and(|s| s.background)) => result,
            };
            match response {
                Err(error) if error.code == ErrorCode::Unauthenticated && attempt == 0 => {
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => return Err(crate::twitch_http::error(ErrorCode::Cancelled)),
                        _ = lease.cancel.cancelled() => return Err(crate::twitch_http::error(ErrorCode::Unauthenticated)),
                        result = self.auth.refresh_rejected(lease.session_id, lease.generation) => result?,
                    }
                }
                Err(error) if error.code == ErrorCode::Unauthenticated => {
                    let _ = self.auth.reject(lease.session_id, lease.generation).await;
                    return Err(error);
                }
                Err(error) => return Err(error),
                Ok(bytes) => {
                    if cancel.is_cancelled() {
                        return Err(error(ErrorCode::Cancelled));
                    }
                    if lease.cancel.is_cancelled() {
                        return Err(error(ErrorCode::Unauthenticated));
                    }
                    let page: Page<T> = decode_page(&bytes)?;
                    // Cache typed public model data only, dropping unknown API
                    // fields (e.g. email) rather than retaining raw responses.
                    let bytes =
                        serde_json::to_vec(&page).map_err(|_| error(ErrorCode::InvalidResponse))?;
                    self.cache
                        .lock()
                        .expect("cache mutex poisoned")
                        .insert(key, bytes, class);
                    return Ok(Cached {
                        value: page,
                        freshness: Freshness::Network,
                        age: Duration::ZERO,
                    });
                }
            }
        }
        unreachable!("at most one refresh per rejected request")
    }
    pub async fn users(
        &self,
        ids: &[String],
        logins: &[String],
        policy: CachePolicy,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<User>>> {
        if ids.len() + logins.len() > 100 {
            return Err(error(ErrorCode::InvalidInput));
        }
        let mut query = params("id", ids)?;
        query.extend(params("login", logins)?);
        self.get("users", query, false, CacheClass::Metadata, policy, cancel)
            .await
    }
    pub async fn account(&self, cancel: &CancellationToken) -> Result<Account> {
        let mut session = None;
        let page: Page<User> = self
            .get_bound(
                "users",
                vec![],
                false,
                CacheClass::Metadata,
                CachePolicy::Fresh,
                cancel,
                &mut session,
            )
            .await?
            .value;
        let mut users = page.data.into_iter();
        let user = users
            .next()
            .ok_or_else(|| error(ErrorCode::InvalidResponse))?;
        if users.next().is_some() {
            return Err(error(ErrorCode::InvalidResponse));
        }
        let session = session.expect("successful request has a session");
        if session.cancel.is_cancelled() {
            return Err(error(ErrorCode::Unauthenticated));
        }
        if user.id != session.user_id {
            return Err(error(ErrorCode::InvalidResponse));
        }
        Ok(user.into())
    }
    pub async fn channels(
        &self,
        ids: &[String],
        policy: CachePolicy,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<Channel>>> {
        if ids.is_empty() {
            return Err(error(ErrorCode::InvalidInput));
        }
        self.get(
            "channels",
            params("broadcaster_id", ids)?,
            false,
            CacheClass::Metadata,
            policy,
            cancel,
        )
        .await
    }
    pub async fn followed_channels(
        &self,
        broadcaster: Option<&str>,
        page: &PageRequest,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<FollowedChannel>>> {
        let mut query = page.query(false)?;
        if let Some(id) = broadcaster {
            query.extend(params("broadcaster_id", &[id.into()])?);
        }
        self.get(
            "channels/followed",
            query,
            true,
            CacheClass::Live,
            CachePolicy::Fresh,
            cancel,
        )
        .await
    }
    pub async fn streams(
        &self,
        filter: &StreamFilter,
        page: &PageRequest,
        policy: CachePolicy,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<Stream>>> {
        let mut query = page.query(true)?;
        for (key, values) in [
            ("user_id", &filter.user_ids),
            ("user_login", &filter.user_logins),
            ("game_id", &filter.game_ids),
            ("language", &filter.languages),
        ] {
            query.extend(params(key, values)?);
        }
        self.get("streams", query, false, CacheClass::Live, policy, cancel)
            .await
    }
    pub async fn followed_streams(
        &self,
        page: &PageRequest,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<Stream>>> {
        self.get(
            "streams/followed",
            page.query(false)?,
            true,
            CacheClass::Live,
            CachePolicy::Fresh,
            cancel,
        )
        .await
    }
    pub async fn games(
        &self,
        ids: &[String],
        names: &[String],
        policy: CachePolicy,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<Game>>> {
        if ids.len() + names.len() == 0 || ids.len() + names.len() > 100 {
            return Err(error(ErrorCode::InvalidInput));
        }
        let mut query = params("id", ids)?;
        query.extend(params("name", names)?);
        self.get(
            "games",
            query,
            false,
            CacheClass::Categories,
            policy,
            cancel,
        )
        .await
    }
    pub async fn top_games(
        &self,
        page: &PageRequest,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<Game>>> {
        self.get(
            "games/top",
            page.query(true)?,
            false,
            CacheClass::Live,
            CachePolicy::Fresh,
            cancel,
        )
        .await
    }
    pub async fn search_categories(
        &self,
        search: &str,
        page: &PageRequest,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<Game>>> {
        let mut query = page.query(false)?;
        query.extend(params("query", &[search.into()])?);
        self.get(
            "search/categories",
            query,
            false,
            CacheClass::Categories,
            CachePolicy::Fresh,
            cancel,
        )
        .await
    }
    pub async fn search_channels(
        &self,
        search: &str,
        live_only: bool,
        page: &PageRequest,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<SearchChannel>>> {
        let mut query = page.query(false)?;
        query.extend(params("query", &[search.into()])?);
        query.push(("live_only".into(), live_only.to_string()));
        self.get(
            "search/channels",
            query,
            false,
            CacheClass::Live,
            CachePolicy::Fresh,
            cancel,
        )
        .await
    }
    pub async fn teams(
        &self,
        selector: TeamSelector<'_>,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<Team>>> {
        let (key, value) = match selector {
            TeamSelector::Id(id) => ("id", id),
            TeamSelector::Name(name) => ("name", name),
        };
        self.get(
            "teams",
            params(key, &[value.into()])?,
            false,
            CacheClass::Metadata,
            CachePolicy::Fresh,
            cancel,
        )
        .await
    }
    pub async fn channel_teams(
        &self,
        broadcaster: &str,
        cancel: &CancellationToken,
    ) -> Result<Cached<Page<ChannelTeam>>> {
        self.get(
            "teams/channel",
            params("broadcaster_id", &[broadcaster.into()])?,
            false,
            CacheClass::Metadata,
            CachePolicy::Fresh,
            cancel,
        )
        .await
    }
    pub async fn users_by_ids(
        &self,
        ids: &[String],
        policy: CachePolicy,
        cancel: &CancellationToken,
    ) -> Result<BatchResult<User>> {
        self.batch(
            ids,
            "users",
            "id",
            CacheClass::Metadata,
            policy,
            cancel,
            |u: &User| &u.id,
        )
        .await
    }
    pub async fn channels_by_ids(
        &self,
        ids: &[String],
        policy: CachePolicy,
        cancel: &CancellationToken,
    ) -> Result<BatchResult<Channel>> {
        self.batch(
            ids,
            "channels",
            "broadcaster_id",
            CacheClass::Metadata,
            policy,
            cancel,
            |c: &Channel| &c.broadcaster_id,
        )
        .await
    }
    pub async fn games_by_ids(
        &self,
        ids: &[String],
        policy: CachePolicy,
        cancel: &CancellationToken,
    ) -> Result<BatchResult<Game>> {
        self.batch(
            ids,
            "games",
            "id",
            CacheClass::Categories,
            policy,
            cancel,
            |g: &Game| &g.id,
        )
        .await
    }
    pub async fn streams_by_user_ids(
        &self,
        ids: &[String],
        policy: CachePolicy,
        cancel: &CancellationToken,
    ) -> Result<BatchResult<Stream>> {
        self.batch(
            ids,
            "streams",
            "user_id",
            CacheClass::Live,
            policy,
            cancel,
            |s: &Stream| &s.user_id,
        )
        .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn batch<T: DeserializeOwned + Serialize>(
        &self,
        ids: &[String],
        endpoint: &str,
        parameter: &str,
        class: CacheClass,
        policy: CachePolicy,
        cancel: &CancellationToken,
        id: impl Fn(&T) -> &str,
    ) -> Result<BatchResult<T>> {
        let batches = id_batches(ids)?;
        let mut result = BatchResult {
            data: Vec::new(),
            missing_ids: Vec::new(),
            stale_ids: Vec::new(),
            incomplete_ids: Vec::new(),
            failures: Vec::new(),
        };
        let mut stopped: Option<crate::domain::AppError> = None;
        let mut session = None;
        for ids in batches {
            if let Some(error) = &stopped {
                result.failures.push(BatchFailure {
                    ids,
                    error: error.clone(),
                });
                continue;
            }
            let mut query = params(parameter, &ids)?;
            if endpoint == "streams" {
                query.push(("first".into(), "100".into()));
            }
            match self
                .get_bound::<T>(endpoint, query, false, class, policy, cancel, &mut session)
                .await
            {
                Ok(page) => {
                    if page.freshness == Freshness::Stale {
                        result.stale_ids.extend(ids.iter().cloned());
                    }
                    let incomplete = page
                        .value
                        .pagination
                        .cursor
                        .as_ref()
                        .is_some_and(|cursor| !cursor.is_empty());
                    if incomplete {
                        result.incomplete_ids.extend(ids.iter().cloned());
                    } else {
                        let returned: HashSet<_> = page.value.data.iter().map(&id).collect();
                        result.missing_ids.extend(
                            ids.into_iter()
                                .filter(|value| !returned.contains(value.as_str())),
                        );
                    }
                    result.data.extend(page.value.data);
                }
                Err(error) => {
                    if matches!(
                        error.code,
                        ErrorCode::Cancelled
                            | ErrorCode::Unauthenticated
                            | ErrorCode::Unauthorized
                            | ErrorCode::RateLimited
                    ) {
                        stopped = Some(error.clone());
                    }
                    result.failures.push(BatchFailure { ids, error });
                }
            }
        }
        Ok(result)
    }
}
fn params(key: &str, values: &[String]) -> Result<Vec<(String, String)>> {
    if values.len() > 100 || values.iter().any(|v| v.is_empty() || v.len() > 128) {
        return Err(error(ErrorCode::InvalidInput));
    }
    let mut seen = HashSet::new();
    Ok(values
        .iter()
        .filter(|v| seen.insert(v.as_str()))
        .map(|v| (key.into(), v.clone()))
        .collect())
}
fn decode_page<T: DeserializeOwned>(bytes: &[u8]) -> Result<Page<T>> {
    let page: Page<T> =
        serde_json::from_slice(bytes).map_err(|_| error(ErrorCode::InvalidResponse))?;
    if page.data.len() > 100
        || page
            .pagination
            .cursor
            .as_ref()
            .is_some_and(|c| c.len() > 2048)
    {
        return Err(error(ErrorCode::InvalidResponse));
    }
    Ok(page)
}
#[cfg(test)]
mod tests;
