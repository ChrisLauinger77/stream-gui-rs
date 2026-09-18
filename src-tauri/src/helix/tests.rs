use super::*;
use crate::{
    credentials::{CredentialStore, Credentials, MemoryCredentialStore},
    test_http::{Reply, Server},
    twitch::api::{DeviceGrant, PollResult, Validation},
};
use std::sync::atomic::{AtomicUsize, Ordering};
use zeroize::Zeroizing;

const USER: &str = include_str!("../../tests/fixtures/helix/user.json");
const CHANNEL: &str = include_str!("../../tests/fixtures/helix/channel.json");
const FOLLOWED: &str = include_str!("../../tests/fixtures/helix/followed.json");
const STREAM: &str = include_str!("../../tests/fixtures/helix/stream.json");
const GAME: &str = include_str!("../../tests/fixtures/helix/game.json");
const SEARCH: &str = include_str!("../../tests/fixtures/helix/search-channel.json");
const TEAM: &str = include_str!("../../tests/fixtures/helix/team.json");
const CHANNEL_TEAM: &str = include_str!("../../tests/fixtures/helix/channel-team.json");

struct TestApi {
    refreshes: Arc<AtomicUsize>,
    gate: Option<Arc<tokio::sync::Notify>>,
}
impl TwitchApi for TestApi {
    async fn begin(&self, _: &str) -> Result<DeviceGrant> {
        Err(error(ErrorCode::Internal))
    }
    async fn poll(&self, _: &str, _: &str) -> Result<PollResult> {
        Ok(PollResult::Pending)
    }
    async fn validate(&self, _: &str) -> Result<Validation> {
        Ok(Validation {
            client_id: "client".into(),
            user_id: "123".into(),
            login: "example".into(),
            scopes: vec!["user:read:follows".into()],
            expires_in: 14400,
        })
    }
    async fn refresh(&self, _: &str, _: &str) -> Result<Credentials> {
        self.refreshes.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = &self.gate {
            gate.notified().await;
        }
        Ok(Credentials {
            access_token: Zeroizing::new("new-access".into()),
            refresh_token: Zeroizing::new("new-refresh".into()),
        })
    }
    async fn revoke(&self, _: &str, _: &str) -> Result<()> {
        Ok(())
    }
}
async fn client(
    server: &Server,
    gate: Option<Arc<tokio::sync::Notify>>,
) -> (HelixClient<TestApi>, Arc<AtomicUsize>) {
    let refreshes = Arc::new(AtomicUsize::new(0));
    let mut store = MemoryCredentialStore::default();
    store
        .save(Credentials {
            access_token: Zeroizing::new("initial-access".into()),
            refresh_token: Zeroizing::new("initial-refresh".into()),
        })
        .unwrap();
    let auth = Arc::new(AuthService::new(
        TestApi {
            refreshes: refreshes.clone(),
            gate,
        },
        Some("client".into()),
        Box::new(store),
    ));
    auth.tick().await.unwrap();
    (
        HelixClient::new(
            TwitchHttp::for_test(&server.base, Duration::from_secs(2)),
            auth,
        ),
        refreshes,
    )
}
#[test]
fn representative_models_preserve_ids_null_fields_and_exclude_private_fields() {
    let users: Page<User> = decode_page(USER.as_bytes()).unwrap();
    assert!(!serde_json::to_string(&users).unwrap().contains("email"));
    let _: Page<Channel> = decode_page(CHANNEL.as_bytes()).unwrap();
    let followed: Page<FollowedChannel> = decode_page(FOLLOWED.as_bytes()).unwrap();
    assert_eq!(followed.total, Some(1));
    let stream: Page<Stream> = decode_page(STREAM.as_bytes()).unwrap();
    assert_eq!(stream.data[0].id, "stream-999999999999999999999");
    assert_eq!(stream.data[0].user_id, "123");
    let _: Page<Game> = decode_page(GAME.as_bytes()).unwrap();
    let _: Page<SearchChannel> = decode_page(SEARCH.as_bytes()).unwrap();
    let team: Page<Team> = decode_page(TEAM.as_bytes()).unwrap();
    assert!(team.data[0].banner.is_none());
    let _: Page<ChannelTeam> = decode_page(CHANNEL_TEAM.as_bytes()).unwrap();
    let account: Account = users.data.into_iter().next().unwrap().into();
    assert_eq!(account.display_name, "Example");
    assert!(account.profile_image_url.is_some());
}
#[tokio::test]
async fn all_endpoint_families_use_typed_paths_and_account_bound_follow_queries() {
    let server = Server::new(
        [
            USER,
            CHANNEL,
            FOLLOWED,
            STREAM,
            STREAM,
            GAME,
            GAME,
            GAME,
            SEARCH,
            TEAM,
            CHANNEL_TEAM,
        ]
        .into_iter()
        .map(|body| Reply::json(200, body))
        .collect(),
    )
    .await;
    let (client, _) = client(&server, None).await;
    let cancel = CancellationToken::new();
    let page = PageRequest::default();
    client.account(&cancel).await.unwrap();
    client
        .channels(&["123".into()], CachePolicy::Fresh, &cancel)
        .await
        .unwrap();
    client
        .followed_channels(None, &page, &cancel)
        .await
        .unwrap();
    client
        .streams(&StreamFilter::default(), &page, CachePolicy::Fresh, &cancel)
        .await
        .unwrap();
    client.followed_streams(&page, &cancel).await.unwrap();
    client
        .games(&["game-1".into()], &[], CachePolicy::Fresh, &cancel)
        .await
        .unwrap();
    client.top_games(&page, &cancel).await.unwrap();
    client
        .search_categories("name & other", &page, &cancel)
        .await
        .unwrap();
    client
        .search_channels("example", true, &page, &cancel)
        .await
        .unwrap();
    client
        .teams(TeamSelector::Name("example"), &cancel)
        .await
        .unwrap();
    client.channel_teams("123", &cancel).await.unwrap();
    let requests = server.requests();
    assert_eq!(requests.len(), 11);
    for (request, path) in requests.iter().zip([
        "users",
        "channels",
        "channels/followed",
        "streams",
        "streams/followed",
        "games",
        "games/top",
        "search/categories",
        "search/channels",
        "teams",
        "teams/channel",
    ]) {
        assert!(request.starts_with(&format!("GET /helix/{path}?")));
    }
    assert!(requests[2].contains("user_id=123"));
    assert!(requests[4].contains("user_id=123"));
    assert!(requests[7].contains("query=name+%26+other"));
}
#[tokio::test]
async fn unauthorized_requests_share_one_refresh_and_retry_with_new_token() {
    let server = Server::new(vec![
        Reply::json(401, "revoked"),
        Reply::json(401, "revoked"),
        Reply::json(200, USER),
        Reply::json(200, USER),
    ])
    .await;
    let gate = Arc::new(tokio::sync::Notify::new());
    let (client, refreshes) = client(&server, Some(gate.clone())).await;
    let cancel = CancellationToken::new();
    let release = async {
        server.wait_for_requests(2).await;
        gate.notify_one();
    };
    let first_ids = ["1".into()];
    let second_ids = ["2".into()];
    let (first, second, ()) = tokio::join!(
        client.users(&first_ids, &[], CachePolicy::Refresh, &cancel),
        client.users(&second_ids, &[], CachePolicy::Refresh, &cancel),
        release
    );
    first.unwrap();
    second.unwrap();
    assert_eq!(refreshes.load(Ordering::SeqCst), 1);
    let requests = server.requests();
    assert_eq!(requests.len(), 4);
    assert!(requests[2].contains("Bearer new-access"));
    assert!(requests[3].contains("Bearer new-access"));
}
#[tokio::test]
async fn repeated_401_clears_authentication_instead_of_looping() {
    let server = Server::new(vec![Reply::json(401, "secret"), Reply::json(401, "secret")]).await;
    let (client, refreshes) = client(&server, None).await;
    assert_eq!(
        client
            .account(&CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unauthenticated
    );
    assert_eq!(refreshes.load(Ordering::SeqCst), 1);
    assert!(client.auth.status().await.user.is_none());
    assert_eq!(server.requests().len(), 2);
}
#[tokio::test]
async fn malformed_models_fail_without_caching_or_exposing_response() {
    let server = Server::new(vec![
        Reply::json(200, r#"{"data":[{"secret":"redact"}]}"#),
        Reply::json(200, USER),
    ])
    .await;
    let (client, _) = client(&server, None).await;
    let error = client.account(&CancellationToken::new()).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidResponse);
    assert!(!error.to_string().contains("redact"));
    client.account(&CancellationToken::new()).await.unwrap();
    assert_eq!(server.requests().len(), 2);
}
#[tokio::test]
async fn cache_hit_invalidation_and_logout_are_backend_owned() {
    let server = Server::new(vec![Reply::json(200, USER), Reply::json(200, USER)]).await;
    let (client, _) = client(&server, None).await;
    let cancel = CancellationToken::new();
    let first = client
        .users(&[], &[], CachePolicy::Fresh, &cancel)
        .await
        .unwrap();
    assert_eq!(first.freshness, Freshness::Network);
    let second = client
        .users(&[], &[], CachePolicy::Fresh, &cancel)
        .await
        .unwrap();
    assert_eq!(second.freshness, Freshness::Cached);
    client.invalidate(CacheClass::Metadata);
    client.account(&cancel).await.unwrap();
    assert_eq!(server.requests().len(), 2);
    client.auth.logout().await.unwrap();
    assert_eq!(
        client.account(&cancel).await.unwrap_err().code,
        ErrorCode::Unauthenticated
    );
    assert_eq!(server.requests().len(), 2);
}
#[tokio::test]
async fn logout_cancels_in_flight_helix_and_does_not_cache_its_response() {
    let server = Server::new(vec![Reply::json(200, USER).delayed(Duration::from_secs(1))]).await;
    let (client, _) = client(&server, None).await;
    let cancel = CancellationToken::new();
    let logout = async {
        server.wait_for_requests(1).await;
        client.auth.logout().await.unwrap();
    };
    let (result, ()) = tokio::join!(client.account(&cancel), logout);
    assert_eq!(result.unwrap_err().code, ErrorCode::Unauthenticated);
    assert!(client.cache.lock().unwrap().is_empty());
}
#[tokio::test]
async fn pagination_is_caller_controlled_and_preserves_cursor_encoding() {
    let first = GAME.replace(
        "\"pagination\": {}",
        "\"pagination\": {\"cursor\":\"next+/=\"}",
    );
    let server = Server::new(vec![Reply::json(200, first), Reply::json(200, GAME)]).await;
    let (client, _) = client(&server, None).await;
    let cancel = CancellationToken::new();
    let first = client
        .top_games(&PageRequest::default(), &cancel)
        .await
        .unwrap()
        .value;
    assert_eq!(server.requests().len(), 1);
    let next = first.next(20).unwrap().unwrap();
    let second = client.top_games(&next, &cancel).await.unwrap().value;
    assert!(second.next(20).unwrap().is_none());
    assert!(server.requests()[1].contains("after=next%2B%2F%3D"));
    assert!(PageRequest::first(0).is_err());
    assert!(PageRequest::first(101).is_err());
    assert!(
        PageRequest::before(20, "cursor")
            .unwrap()
            .query(false)
            .is_err()
    );
}
#[tokio::test]
async fn batching_reports_partial_failure_and_missing_ids_without_unlimited_fetches() {
    let server = Server::new(vec![
        Reply::json(200, USER.replace("\"123\"", "\"0\"")),
        Reply::json(503, "down"),
        Reply::json(503, "down"),
    ])
    .await;
    let (client, _) = client(&server, None).await;
    let ids = (0..101).map(|i| i.to_string()).collect::<Vec<_>>();
    let result = client
        .users_by_ids(&ids, CachePolicy::Fresh, &CancellationToken::new())
        .await
        .unwrap();
    assert!(!result.complete());
    assert_eq!(result.data.len(), 1);
    assert_eq!(result.missing_ids.len(), 99);
    assert_eq!(result.failures.len(), 1);
    assert_eq!(result.failures[0].ids, ["100"]);
    assert_eq!(server.requests().len(), 3);
}
#[test]
fn id_batch_boundaries_duplicates_and_query_limits() {
    for (count, expected) in [(0, 0), (1, 1), (100, 1), (101, 2), (1000, 10)] {
        let ids = (0..count).map(|i| i.to_string()).collect::<Vec<_>>();
        assert_eq!(id_batches(&ids).unwrap().len(), expected);
    }
    assert_eq!(
        id_batches(&["123".into(), "123".into()]).unwrap(),
        vec![vec!["123".to_string()]]
    );
    assert!(id_batches(&vec!["a".into(); 1001]).is_err());
    assert!(id_batches(&[String::new()]).is_err());
    let ids = (0..100)
        .map(|i| format!("{}{}", "&".repeat(100), i))
        .collect::<Vec<_>>();
    let batches = id_batches(&ids).unwrap();
    assert_eq!(batches.iter().map(Vec::len).sum::<usize>(), ids.len());
    for batch in batches {
        let query = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(batch.iter().map(|id| ("broadcaster_id", id)))
            .finish();
        assert!(
            query.len() <= 6000,
            "Encoded query exceeded the batch budget"
        );
    }
}
#[tokio::test(start_paused = true)]
async fn cache_expiry_staleness_bounds_and_invalidation() {
    let mut cache = TwitchCache::default();
    assert!(cache.get("missing", CachePolicy::Fresh).is_none());
    for class in [
        CacheClass::Live,
        CacheClass::Metadata,
        CacheClass::Categories,
    ] {
        cache.insert("key".into(), b"data".to_vec(), class);
        assert_eq!(
            cache.get("key", CachePolicy::Fresh).unwrap().freshness,
            Freshness::Cached
        );
        assert!(cache.get("key", CachePolicy::Refresh).is_none());
        tokio::time::advance(class.ttl()).await;
        assert!(cache.get("key", CachePolicy::Fresh).is_none());
        assert_eq!(
            cache.get("key", CachePolicy::AllowStale).unwrap().freshness,
            Freshness::Stale
        );
        cache.invalidate(class);
        assert!(cache.is_empty());
    }
    for n in 0..200 {
        cache.insert(n.to_string(), vec![0; 100_000], CacheClass::Live);
    }
    assert!(cache.len() <= 83);
    cache.clear();
    assert!(cache.is_empty());
}

#[tokio::test]
async fn incomplete_batch_does_not_report_unseen_ids_as_missing() {
    let server = Server::new(vec![Reply::json(
        200,
        r#"{"data":[],"pagination":{"cursor":"next"}}"#,
    )])
    .await;
    let (client, _) = client(&server, None).await;
    let result = client
        .streams_by_user_ids(
            &["123".into()],
            CachePolicy::Fresh,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert!(!result.complete());
    assert_eq!(result.incomplete_ids, ["123"]);
    assert!(result.missing_ids.is_empty());
}

#[tokio::test]
async fn delayed_401_retry_cannot_cross_logout_and_new_login() {
    delayed_401_retry_after_logout(true).await;
}

#[tokio::test]
async fn delayed_401_retry_terminates_after_logout_without_relogin() {
    delayed_401_retry_after_logout(false).await;
}

async fn delayed_401_retry_after_logout(relogin: bool) {
    fn validation(id: &str) -> String {
        serde_json::json!({"client_id":"client", "user_id":id,"login":"synthetic-user","scopes":["user:read:follows"],"expires_in":14400}).to_string()
    }
    let token = r#"{"access_token":"synthetic-access","refresh_token":"synthetic-refresh","token_type":"bearer"}"#;
    let gate = Arc::new(tokio::sync::Notify::new());
    let server = Server::new(vec![
        Reply::json(200, validation("account-a")),
        Reply::json(401, "synthetic unauthorized"),
        Reply::json(200, token).gated(gate.clone()),
        Reply::json(200, validation("account-a")),
        Reply::json(200, "{}"),
        Reply::json(200, r#"{"device_code":"synthetic-device","user_code":"CODE","verification_uri":"https://www.twitch.tv/activate","expires_in":1800,"interval":5}"#),
        Reply::json(200, token),
        Reply::json(200, validation("account-b")),
        Reply::json(200, STREAM),
    ]).await;
    let http = TwitchHttp::for_test(&server.base, Duration::from_secs(10));
    let mut store = MemoryCredentialStore::default();
    store
        .save(Credentials {
            access_token: Zeroizing::new("synthetic-initial-access".into()),
            refresh_token: Zeroizing::new("synthetic-initial-refresh".into()),
        })
        .unwrap();
    let auth = Arc::new(AuthService::new(
        HttpTwitchApi::with_http(&http),
        Some("client".into()),
        Box::new(store),
    ));
    auth.tick().await.unwrap();
    let original = auth.lease().await.unwrap();
    let client = HelixClient::new(http, auth.clone());
    let cancel = CancellationToken::new();
    let page = PageRequest::default();
    let pending = client.followed_streams(&page, &cancel);
    tokio::pin!(pending);
    tokio::select! {
        result = &mut pending => panic!("request completed before gated refresh: {result:?}"),
        _ = server.wait_for_requests(3) => {},
    }
    // Stop polling the caller. Its owned rotation still finishes and persists
    // before logout and the possible replacement login acquire the auth lock.
    gate.notify_one();
    let rotated = auth.lease().await.unwrap();
    assert_eq!(&*rotated.token, "synthetic-access");
    assert_eq!(rotated.session_id, original.session_id);
    assert_ne!(rotated.generation, original.generation);
    auth.logout().await.unwrap();
    if relogin {
        auth.login().await.unwrap();
        tokio::time::sleep(Duration::from_secs(5)).await;
        auth.tick().await.unwrap();
        assert_eq!(auth.status().await.user.unwrap().id, "account-b");
    }
    assert!(original.cancel.is_cancelled());
    assert_eq!(pending.await.unwrap_err().code, ErrorCode::Unauthenticated);
    // Even callers queued behind the replacement login cannot validate/refresh
    // or reacquire a lease for that new session on behalf of the old one.
    assert_eq!(
        auth.lease_for_session(original.session_id)
            .await
            .err()
            .unwrap()
            .code,
        ErrorCode::Unauthenticated
    );
    assert_eq!(
        auth.refresh_rejected(original.session_id, original.generation)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unauthenticated
    );
    let requests = server.requests();
    assert_eq!(requests.len(), if relogin { 8 } else { 5 });
    let helix: Vec<_> = requests
        .iter()
        .filter(|request| request.starts_with("GET /helix/"))
        .collect();
    assert_eq!(helix.len(), 1);
    assert!(helix[0].contains("user_id=account-a"));
    assert!(client.cache.lock().unwrap().is_empty());
}
