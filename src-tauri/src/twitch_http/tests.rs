use super::*;
use crate::test_http::{Reply, Server};
use reqwest::header::HeaderMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn headers_queries_and_response_bodies_are_correct() {
    let server = Server::new(vec![Reply::json(200, r#"{"data":[]}"#)]).await;
    let http = TwitchHttp::for_test(&server.base, Duration::from_secs(1));
    let body = http
        .get(
            "users",
            &[("id".into(), "a&b".into()), ("id".into(), "second".into())],
            "public-client",
            "test-secret",
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(body, br#"{"data":[]}"#);
    let requests = server.requests();
    let request = requests[0].to_lowercase();
    assert!(request.starts_with("get /helix/users?id=a%26b&id=second "));
    assert!(request.contains("authorization: bearer test-secret"));
    assert!(request.contains("client-id: public-client"));
}
#[tokio::test]
async fn terminal_http_errors_are_redacted_and_not_retried() {
    for (status, expected) in [
        (401, ErrorCode::Unauthenticated),
        (403, ErrorCode::Unauthorized),
        (429, ErrorCode::RateLimited),
        (400, ErrorCode::InvalidInput),
        (501, ErrorCode::TwitchServer),
    ] {
        let server = Server::new(vec![Reply::json(status, r#"{"message":"secret-token"}"#)]).await;
        let http = TwitchHttp::for_test(&server.base, Duration::from_secs(1));
        let error = http
            .get(
                "users",
                &[],
                "client",
                "secret-token",
                &CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, expected);
        assert!(
            !serde_json::to_string(&error)
                .unwrap()
                .contains("secret-token")
        );
        assert_eq!(server.requests().len(), 1);
    }
}
#[tokio::test]
async fn response_headers_update_the_shared_rate_budget() {
    let reset = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 60;
    let server = Server::new(vec![
        Reply::json(200, "ok")
            .header("Ratelimit-Limit", "800")
            .header("Ratelimit-Remaining", "799")
            .header("Ratelimit-Reset", reset.to_string()),
    ])
    .await;
    let http = TwitchHttp::for_test(&server.base, Duration::from_secs(1));
    http.get("users", &[], "client", "token", &CancellationToken::new())
        .await
        .unwrap();
    let budget = http.rate.snapshot();
    assert_eq!(budget.limit, Some(800));
    assert_eq!(budget.remaining, Some(799));
    assert_eq!(budget.reset_at, Some(reset));
    assert_eq!(budget.in_flight, 0);
}
#[tokio::test]
async fn server_failure_has_one_bounded_retry() {
    let server = Server::new(vec![Reply::json(503, "temporary"), Reply::json(200, "ok")]).await;
    let http = TwitchHttp::for_test(&server.base, Duration::from_secs(1));
    assert_eq!(
        http.get("users", &[], "client", "token", &CancellationToken::new())
            .await
            .unwrap(),
        b"ok"
    );
    assert_eq!(server.requests().len(), 2);
    let server = Server::new(vec![
        Reply::json(500, "temporary"),
        Reply::json(502, "still down"),
    ])
    .await;
    let http = TwitchHttp::for_test(&server.base, Duration::from_secs(1));
    assert_eq!(
        http.get("users", &[], "client", "token", &CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        ErrorCode::TwitchServer
    );
    assert_eq!(server.requests().len(), 2);
}
#[tokio::test]
async fn timeout_cancellation_and_network_failures_are_distinct() {
    let server = Server::new(
        (0..3)
            .map(|_| Reply::json(200, "late").delayed(Duration::from_secs(1)))
            .collect(),
    )
    .await;
    let http = TwitchHttp::for_test(&server.base, Duration::from_millis(30));
    assert_eq!(
        http.get("users", &[], "client", "token", &CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        ErrorCode::Timeout
    );
    let cancel = CancellationToken::new();
    let request = http.get("users", &[], "client", "token", &cancel);
    tokio::pin!(request);
    let cancel_later = async {
        server.wait_for_requests(3).await;
        cancel.cancel();
    };
    let (result, ()) = tokio::join!(request, cancel_later);
    assert_eq!(result.unwrap_err().code, ErrorCode::Cancelled);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let http = TwitchHttp::for_test(&base, Duration::from_millis(50));
    assert_eq!(
        http.get("users", &[], "client", "token", &CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        ErrorCode::Network
    );
}
fn headers(remaining: u32, reset: u64) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("ratelimit-limit", "10".parse().unwrap());
    headers.insert(
        "ratelimit-remaining",
        remaining.to_string().parse().unwrap(),
    );
    headers.insert("ratelimit-reset", reset.to_string().parse().unwrap());
    headers
}
#[tokio::test(start_paused = true)]
async fn shared_budget_reserves_concurrently_without_overspending_and_resets() {
    let rate = Arc::new(RateLimiter::default());
    let reset = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 10;
    rate.observe(&headers(2, reset), false);
    let cancel = CancellationToken::new();
    let first = rate.reserve(&cancel).await.unwrap();
    let second = rate.reserve(&cancel).await.unwrap();
    assert_eq!(rate.snapshot().remaining, Some(0));
    let third = rate.reserve(&cancel);
    tokio::pin!(third);
    assert!(
        tokio::time::timeout(Duration::from_millis(1), &mut third)
            .await
            .is_err()
    );
    // An out-of-order response cannot increase a budget already reserved.
    rate.observe(&headers(9, reset), false);
    assert_eq!(rate.snapshot().remaining, Some(0));
    drop(first);
    drop(second);
    tokio::time::advance(Duration::from_secs(10)).await;
    let _third = third.await.unwrap();
    assert_eq!(rate.snapshot().in_flight, 1);
}
#[tokio::test(start_paused = true)]
async fn rate_limit_429_blocks_shared_budget_and_wait_is_cancellable() {
    let rate = Arc::new(RateLimiter::default());
    rate.observe(&HeaderMap::new(), true);
    assert_eq!(rate.snapshot().remaining, Some(0));
    let cancel = CancellationToken::new();
    let request = rate.reserve(&cancel);
    tokio::pin!(request);
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut request)
            .await
            .is_err()
    );
    cancel.cancel();
    assert_eq!(request.await.err().unwrap().code, ErrorCode::Cancelled);
    tokio::time::advance(Duration::from_secs(1)).await;
    assert!(rate.reserve(&CancellationToken::new()).await.is_ok());
}
#[tokio::test]
async fn malformed_or_oversized_json_is_not_reflected_in_errors() {
    for body in ["secret-content".into(), "x".repeat(65 * 1024)] {
        let server = Server::new(vec![Reply::json(200, body)]).await;
        let http = TwitchHttp::for_test(&server.base, Duration::from_secs(1));
        let response = http.client.get(&server.base).send().await.unwrap();
        let error = decode::<serde_json::Value>(response, 64 * 1024)
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidResponse);
        assert!(!error.to_string().contains("secret-content"));
    }
}
