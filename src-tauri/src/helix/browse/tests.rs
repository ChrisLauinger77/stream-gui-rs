use super::*;
use crate::{
    credentials::{CredentialStore, Credentials, MemoryCredentialStore},
    test_http::{Reply, Server},
    twitch::api::{DeviceGrant, PollResult, Validation},
};
use zeroize::Zeroizing;
const USER: &str = include_str!("../../../tests/fixtures/helix/user.json");
const CHANNEL: &str = include_str!("../../../tests/fixtures/helix/channel.json");
const FOLLOWED: &str = include_str!("../../../tests/fixtures/helix/followed.json");
const STREAM: &str = include_str!("../../../tests/fixtures/helix/stream.json");
const GAME: &str = include_str!("../../../tests/fixtures/helix/game.json");
struct Api;
impl TwitchApi for Api {
    async fn begin(&self, _: &str) -> Result<DeviceGrant> {
        Err(error(ErrorCode::Internal))
    }
    async fn poll(&self, _: &str, _: &str) -> Result<PollResult> {
        Ok(PollResult::Pending)
    }
    async fn validate(&self, _: &str) -> Result<Validation> {
        Ok(Validation {
            client_id: "client".into(),
            user_id: "viewer".into(),
            login: "viewer".into(),
            scopes: vec!["user:read:follows".into()],
            expires_in: 14400,
        })
    }
    async fn refresh(&self, _: &str, _: &str) -> Result<Credentials> {
        Err(error(ErrorCode::AuthInvalid))
    }
    async fn revoke(&self, _: &str, _: &str) -> Result<()> {
        Ok(())
    }
}
async fn client(replies: Vec<Reply>) -> (Server, HelixClient<Api>, BrowseRequest) {
    let server = Server::new(replies).await;
    let mut store = MemoryCredentialStore::default();
    store
        .save(Credentials {
            access_token: Zeroizing::new("synthetic-access".into()),
            refresh_token: Zeroizing::new("synthetic-refresh".into()),
        })
        .unwrap();
    let auth = Arc::new(AuthService::new(
        Api,
        Some("client".into()),
        Box::new(store),
    ));
    auth.tick().await.unwrap();
    let page = BrowseRequest {
        session_id: auth.status().await.session_id.unwrap(),
        cursor: None,
        refresh: false,
    };
    let http = TwitchHttp::for_test(&server.base, Duration::from_secs(3));
    (server, HelixClient::new(http, auth), page)
}
#[tokio::test]
async fn followed_streams_preserve_ids_cursor_and_backend_cache_refresh() {
    let body = STREAM.replace(
        "\"pagination\": {}",
        "\"pagination\": {\"cursor\":\"next+/=\"}",
    );
    let (server, client, mut request) =
        client(vec![Reply::json(200, body), Reply::json(200, STREAM)]).await;
    let cancel = CancellationToken::new();
    let page = client
        .browse_followed_streams(request.clone(), &cancel)
        .await
        .unwrap();
    assert_ne!(page.items[0].stream_id, page.items[0].broadcaster_id);
    assert_eq!(page.cursor.as_deref(), Some("next+/="));
    assert!(
        page.items[0]
            .preview_url
            .as_ref()
            .unwrap()
            .ends_with("480x270.jpg")
    );
    assert!(page.items[0].category_id.is_none());
    assert_eq!(
        client
            .browse_followed_streams(request.clone(), &cancel)
            .await
            .unwrap()
            .freshness,
        DataFreshness::Cached
    );
    assert_eq!(server.requests().len(), 1);
    request.refresh = true;
    request.cursor = page.cursor;
    client
        .browse_followed_streams(request, &cancel)
        .await
        .unwrap();
    assert!(
        server.requests()[0].starts_with("GET /helix/streams/followed?first=30&user_id=viewer")
    );
    assert!(server.requests()[1].contains("after=next%2B%2F%3D"));
    client.clear_cache();
    assert!(client.cache.lock().unwrap().is_empty());
}
#[tokio::test]
async fn followed_channels_enrich_one_bounded_batch_and_confirm_offline() {
    let (server, client, request) = client(vec![
        Reply::json(200, FOLLOWED),
        Reply::json(200, USER),
        Reply::json(200, r#"{"data":[]}"#),
    ])
    .await;
    let page = client
        .browse_followed_channels(request, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(page.items[0].live_state, LiveState::Offline);
    assert!(page.items[0].followed_at.is_some());
    assert!(page.items[0].image_url.is_some());
    assert_eq!(server.requests().len(), 3);
    assert!(server.requests()[1].contains("users?id=123"));
    assert!(server.requests()[2].contains("streams?user_id=123&first=100"));
}
#[tokio::test]
async fn failed_live_enrichment_is_unknown_and_missing_metadata_keeps_follow_identity() {
    let (_server, client, request) = client(vec![
        Reply::json(200, FOLLOWED),
        Reply::json(200, r#"{"data":[]}"#),
        Reply::json(503, "private-body"),
        Reply::json(503, "private-body"),
    ])
    .await;
    let page = client
        .browse_followed_channels(request, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(page.items[0].display_name, "Example");
    assert!(page.items[0].image_url.is_none());
    assert_eq!(page.items[0].live_state, LiveState::Unknown);
    assert_eq!(page.warnings, [ErrorCode::TwitchServer]);
    assert!(
        !serde_json::to_string(&page)
            .unwrap()
            .contains("private-body")
    );
}
#[tokio::test]
async fn category_metadata_and_cursor_are_bound_to_the_selected_category() {
    let body = STREAM.replace(
        "\"pagination\": {}",
        "\"pagination\": {\"cursor\":\"more\"}",
    );
    let (server, client, page) = client(vec![Reply::json(200, GAME), Reply::json(200, body)]).await;
    let details = client
        .browse_category(
            EntityRequest {
                page,
                id: "game-1".into(),
            },
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(details.category.id, "game-1");
    assert_eq!(details.streams.cursor.as_deref(), Some("more"));
    assert!(server.requests()[1].contains("game_id=game-1"));
    assert!(details.category.image_url.unwrap().ends_with("144x192.jpg"));
}
#[tokio::test]
async fn channel_details_distinguish_live_offline_and_unavailable() {
    for (body, state) in [
        (STREAM, LiveState::Live),
        (r#"{"data":[]}"#, LiveState::Offline),
        ("error", LiveState::Unknown),
    ] {
        let replies = if state == LiveState::Unknown {
            vec![
                Reply::json(200, USER),
                Reply::json(200, CHANNEL),
                Reply::json(503, body),
                Reply::json(503, body),
            ]
        } else {
            vec![
                Reply::json(200, USER),
                Reply::json(200, CHANNEL),
                Reply::json(200, body),
            ]
        };
        let (_server, client, page) = client(replies).await;
        let details = client
            .browse_channel(
                EntityRequest {
                    page,
                    id: "123".into(),
                },
                &CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(details.channel.live_state, state);
        assert_eq!(details.stream.is_some(), state == LiveState::Live);
        assert_eq!(details.description.as_deref(), Some("Public description"));
        assert!(!serde_json::to_string(&details).unwrap().contains("email"));
    }
}
#[tokio::test]
async fn search_trims_encodes_and_rejects_empty_or_oversized_queries() {
    let (server, client, page) = client(vec![Reply::json(200, GAME)]).await;
    for query in [String::new(), "  ".into(), "a".repeat(101)] {
        assert_eq!(
            client
                .browse_search_categories(
                    SearchRequest {
                        page: page.clone(),
                        query
                    },
                    &CancellationToken::new()
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
    client
        .browse_search_categories(
            SearchRequest {
                page,
                query: " a & b ".into(),
            },
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(server.requests().len(), 1);
    assert!(server.requests()[0].contains("query=a+%26+b"));
}
#[tokio::test]
async fn logout_during_enrichment_prevents_further_dispatch_and_old_session_queries() {
    let gate = Arc::new(tokio::sync::Notify::new());
    let (server, client, page) = client(vec![
        Reply::json(200, FOLLOWED),
        Reply::json(200, USER).gated(gate.clone()),
        Reply::json(200, STREAM),
    ])
    .await;
    let cancel = CancellationToken::new();
    let pending = client.browse_followed_channels(page.clone(), &cancel);
    let logout = async {
        server.wait_for_requests(2).await;
        client.auth.logout().await.unwrap();
        gate.notify_one();
    };
    let (result, ()) = tokio::join!(pending, logout);
    assert_eq!(result.unwrap_err().code, ErrorCode::Unauthenticated);
    assert_eq!(
        client.browse_streams(page, &cancel).await.unwrap_err().code,
        ErrorCode::Unauthenticated
    );
    assert_eq!(server.requests().len(), 2);
    assert!(client.auth.status().await.session_id.is_none());
}
#[tokio::test]
async fn unauthorized_enrichment_is_terminal_and_never_offline() {
    let (_server, client, page) =
        client(vec![Reply::json(200, FOLLOWED), Reply::json(403, "secret")]).await;
    assert_eq!(
        client
            .browse_followed_channels(page, &CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unauthorized
    );
}
#[test]
fn images_reject_foreign_urls_and_request_bounded_dimensions() {
    assert!(image_url("https://evil.invalid/image", 480, 270).is_none());
    assert!(image_url("https://user@static-cdn.jtvnw.net/a", 480, 270).is_none());
    assert!(image_url("javascript:alert(1)", 480, 270).is_none());
    assert!(
        image_url("https://static-cdn.jtvnw.net/a-300x300.png", 70, 70)
            .unwrap()
            .ends_with("70x70.png")
    );
}

#[tokio::test]
async fn followed_channels_reuse_fresh_live_matches_but_refresh_rechecks_them() {
    let (server, client, request) = client(vec![
        Reply::json(200, STREAM),
        Reply::json(200, FOLLOWED),
        Reply::json(200, USER),
        Reply::json(200, FOLLOWED),
        Reply::json(200, r#"{"data":[]}"#),
    ])
    .await;
    let cancel = CancellationToken::new();
    client
        .browse_followed_streams(request.clone(), &cancel)
        .await
        .unwrap();
    let result = client
        .browse_followed_channels(request.clone(), &cancel)
        .await
        .unwrap();
    assert_eq!(result.items[0].live_state, LiveState::Live);
    assert_eq!(result.freshness, DataFreshness::Cached);
    assert_eq!(server.requests().len(), 3);
    let result = client
        .browse_followed_channels(
            BrowseRequest {
                refresh: true,
                ..request
            },
            &cancel,
        )
        .await
        .unwrap();
    assert_eq!(result.items[0].live_state, LiveState::Offline);
    assert_eq!(server.requests().len(), 5);
}
#[tokio::test]
async fn followed_stream_page_omissions_do_not_establish_offline_state() {
    let (server, client, request) = client(vec![
        Reply::json(200, r#"{"data":[],"pagination":{"cursor":"more"}}"#),
        Reply::json(200, FOLLOWED),
        Reply::json(200, USER),
        Reply::json(200, STREAM),
    ])
    .await;
    let cancel = CancellationToken::new();
    client
        .browse_followed_streams(request.clone(), &cancel)
        .await
        .unwrap();
    let result = client
        .browse_followed_channels(request, &cancel)
        .await
        .unwrap();
    assert_eq!(result.items[0].live_state, LiveState::Live);
    assert_eq!(server.requests().len(), 4);
}

#[tokio::test]
async fn playback_resolves_live_identity_in_rust_and_rechecks_instead_of_using_cache() {
    let renamed = STREAM.replace("\"example\"", "\"new_login\"");
    let (server, client, page) = client(vec![
        Reply::json(200, STREAM),
        Reply::json(200, renamed),
        Reply::json(200, r#"{"data":[]}"#),
    ])
    .await;
    let cancel = CancellationToken::new();
    let first = client
        .playback_stream(page.session_id.clone(), "123".into(), &cancel)
        .await
        .unwrap();
    assert_eq!(first.login, "example");
    assert_ne!(first.stream_id, first.broadcaster_id);
    let next = client
        .playback_stream(page.session_id.clone(), "123".into(), &cancel)
        .await
        .unwrap();
    assert_eq!(next.login, "new_login");
    assert_eq!(
        client
            .playback_stream(page.session_id, "123".into(), &cancel)
            .await
            .unwrap_err()
            .code,
        ErrorCode::StreamOffline
    );
    assert_eq!(server.requests().len(), 3);
    assert!(server.requests()[0].starts_with("GET /helix/streams?user_id=123&first=1"));
}

#[tokio::test]
async fn playback_rejects_wrong_identity_malformed_login_and_stale_authentication() {
    let bad_login = STREAM.replace("\"example\"", "\"bad/--player\"");
    let (server, client, page) =
        client(vec![Reply::json(200, STREAM), Reply::json(200, bad_login)]).await;
    let cancel = CancellationToken::new();
    assert_eq!(
        client
            .playback_stream(page.session_id.clone(), "456".into(), &cancel)
            .await
            .unwrap_err()
            .code,
        ErrorCode::StreamOffline
    );
    assert_eq!(
        client
            .playback_stream(page.session_id.clone(), "123".into(), &cancel)
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    client.auth.logout().await.unwrap();
    assert_eq!(
        client
            .playback_stream(page.session_id, "123".into(), &cancel)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unauthenticated
    );
    assert_eq!(server.requests().len(), 2);
}

#[tokio::test]
async fn logout_during_playback_identity_lookup_cancels_the_pending_launch() {
    let gate = Arc::new(tokio::sync::Notify::new());
    let (server, client, page) = client(vec![Reply::json(200, STREAM).gated(gate.clone())]).await;
    let cancel = CancellationToken::new();
    let lookup = client.playback_stream(page.session_id, "123".into(), &cancel);
    let logout = async {
        server.wait_for_requests(1).await;
        client.auth.logout().await.unwrap();
        gate.notify_one();
    };
    let (result, ()) = tokio::join!(lookup, logout);
    assert_eq!(result.unwrap_err().code, ErrorCode::Unauthenticated);
}

#[tokio::test]
async fn chat_identity_uses_fresh_bound_users_including_offline_channels() {
    let renamed = USER.replace("\"example\"", "\"new_login\"");
    let (server, client, page) = client(vec![
        Reply::json(200, USER),
        Reply::json(200, renamed),
        Reply::json(200, USER),
    ])
    .await;
    let cancel = CancellationToken::new();
    assert_eq!(
        client
            .chat_login(page.session_id.clone(), "123".into(), &cancel)
            .await
            .unwrap(),
        "example"
    );
    assert_eq!(
        client
            .chat_login(page.session_id.clone(), "123".into(), &cancel)
            .await
            .unwrap(),
        "new_login"
    );
    assert_eq!(
        client
            .chat_login(page.session_id.clone(), "456".into(), &cancel)
            .await
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    assert!(
        client
            .chat_login(page.session_id, "url/attack".into(), &cancel)
            .await
            .is_err()
    );
    assert_eq!(server.requests().len(), 3);
    assert!(
        server
            .requests()
            .iter()
            .all(|request| request.starts_with("GET /helix/users?id="))
    );
}

#[tokio::test]
async fn chat_identity_rejects_url_metadata_and_logout_during_lookup() {
    let (server, client, page) = client(vec![Reply::json(
        200,
        USER.replace("\"example\"", "\"evil.example/path\""),
    )])
    .await;
    assert_eq!(
        client
            .chat_login(page.session_id, "123".into(), &CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    assert_eq!(server.requests().len(), 1);
    let gate = Arc::new(tokio::sync::Notify::new());
    let (server, client, page) =
        self::client(vec![Reply::json(200, USER).gated(gate.clone())]).await;
    let cancel = CancellationToken::new();
    let lookup = client.chat_login(page.session_id.clone(), "123".into(), &cancel);
    let logout = async {
        server.wait_for_requests(1).await;
        client.auth.logout().await.unwrap();
        gate.notify_one();
    };
    let (result, ()) = tokio::join!(lookup, logout);
    assert_eq!(result.unwrap_err().code, ErrorCode::Unauthenticated);
    assert_eq!(
        client
            .chat_login(page.session_id, "123".into(), &cancel)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unauthenticated
    );
    assert_eq!(server.requests().len(), 1);
}
