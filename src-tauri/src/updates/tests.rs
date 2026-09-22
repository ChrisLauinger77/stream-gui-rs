use super::*;
use crate::test_http::{Reply, Server};
fn release(tag: &str) -> serde_json::Value {
    serde_json::json!({"tag_name": tag, "html_url": format!("{RELEASE_PREFIX}{tag}"), "draft":false, "prerelease":false, "name":"<script>malicious</script>", "body":"<img src=evil onerror=bad>"})
}
#[test]
fn semantic_comparison_and_untrusted_release_fields() {
    for (current, latest, phase) in [
        ("0.3.0", "v0.4.0", UpdatePhase::Available),
        ("0.3.0", "v0.3.0", UpdatePhase::Current),
        ("0.4.0", "v0.4.0", UpdatePhase::Current),
        ("0.4.0", "v0.3.0", UpdatePhase::Development),
        ("0.4.0-dev", "v0.3.0", UpdatePhase::Development),
        ("0.9.0", "v0.10.0", UpdatePhase::Available),
        ("0.3.0+local", "v0.3.0", UpdatePhase::Current),
    ] {
        let status =
            parse_release(&serde_json::to_vec(&release(latest)).unwrap(), current).unwrap();
        assert_eq!(status.phase, phase);
        assert_eq!(status.latest_version.as_deref(), latest.strip_prefix('v'));
    }
    for key in ["draft", "prerelease"] {
        let mut value = release("v9.0.0");
        value[key] = true.into();
        assert_eq!(
            parse_release(&serde_json::to_vec(&value).unwrap(), "0.3.0")
                .unwrap()
                .phase,
            UpdatePhase::NoStableRelease
        );
    }
    assert_eq!(
        parse_release(
            &serde_json::to_vec(&release("v9.0.0-rc.1")).unwrap(),
            "0.3.0"
        )
        .unwrap()
        .phase,
        UpdatePhase::NoStableRelease
    );
    for tag in [
        "main",
        "0.4.0",
        "v0.04.0",
        "v9.0.0+bad",
        "v1.0.0/../../evil",
        "v<svg>",
    ] {
        assert!(parse_release(&serde_json::to_vec(&release(tag)).unwrap(), "0.3.0").is_err());
        assert!(release_url(tag).is_err());
    }
    for url in [
        "http://github.com/ChrisLauinger77/stream-gui-rs/releases/tag/v0.4.0",
        "https://evil.example/v0.4.0",
        "https://github.com/other/repo/releases/tag/v0.4.0",
        "https://github.com/ChrisLauinger77/stream-gui-rs/releases/tag/v0.4.0?evil=true",
    ] {
        let mut value = release("v0.4.0");
        value["html_url"] = url.into();
        assert!(parse_release(&serde_json::to_vec(&value).unwrap(), "0.3.0").is_err());
    }
}
fn service(server: &Server) -> Arc<Updates> {
    Arc::new(Updates {
        endpoint: server.base.clone(),
        timeout: Duration::from_millis(200),
        clock: Clock::for_test(),
        ..Default::default()
    })
}
#[tokio::test]
async fn cache_refresh_ttl_headers_and_fixed_destination() {
    let body = release("v99.0.0").to_string();
    let server = Server::new(vec![
        Reply::json(200, &body),
        Reply::json(200, &body),
        Reply::json(200, &body),
    ])
    .await;
    let updates = service(&server);
    assert_eq!(updates.status().phase, UpdatePhase::NotChecked);
    assert!(server.requests().is_empty());
    assert!(updates.release_destination().is_err());
    assert_eq!(updates.check(false).await.phase, UpdatePhase::Available);
    updates.check(true).await;
    assert_eq!(server.requests().len(), 1);
    updates.clock.advance_wall(REFRESH_INTERVAL);
    updates.check(false).await;
    assert_eq!(server.requests().len(), 1);
    updates.check(true).await;
    assert_eq!(server.requests().len(), 2);
    updates.clock.advance_wall(TTL);
    updates.check(false).await;
    assert_eq!(server.requests().len(), 3);
    assert_eq!(
        updates.release_destination().unwrap(),
        format!("{RELEASE_PREFIX}v99.0.0")
    );
    assert_eq!(
        ENDPOINT,
        "https://api.github.com/repos/ChrisLauinger77/stream-gui-rs/releases/latest"
    );
    for request in server.requests() {
        let request = request.to_lowercase();
        assert!(!request.contains("authorization"));
        assert!(!request.contains("client-id"));
        assert!(!request.contains("cookie"));
    }
}
#[tokio::test]
async fn failures_are_quiet_cached_bounded_and_never_redirect_or_retry() {
    for reply in [
        Reply::json(200, "broken"),
        Reply::json(200, "x".repeat(MAX_BODY + 1)),
        Reply::json(500, "secret provider body"),
        Reply::json(403, "rate limited"),
        Reply::disconnect(),
        Reply::json(200, release("v9.0.0").to_string()).delayed(Duration::from_secs(1)),
        Reply::json(302, "").header("Location", "https://evil.invalid"),
    ] {
        let server = Server::new(vec![reply]).await;
        let updates = service(&server);
        assert_eq!(updates.check(false).await.phase, UpdatePhase::Unavailable);
        updates.check(false).await;
        updates.check(true).await;
        assert_eq!(server.requests().len(), 1);
        assert!(updates.release_destination().is_err());
    }
}
#[tokio::test]
async fn concurrent_and_dropped_callers_share_service_owned_check_and_shutdown_cancels() {
    let gate = Arc::new(tokio::sync::Notify::new());
    let server = Server::new(vec![
        Reply::json(200, release("v9.0.0").to_string()).gated(gate.clone()),
    ])
    .await;
    let updates = Arc::new(Updates {
        endpoint: server.base.clone(),
        clock: Clock::for_test(),
        ..Default::default()
    });
    let task = {
        let updates = updates.clone();
        tokio::spawn(async move { updates.check(true).await })
    };
    server.wait_for_requests(1).await;
    task.abort();
    assert_eq!(updates.check(true).await.phase, UpdatePhase::Checking);
    gate.notify_one();
    tokio::time::timeout(Duration::from_secs(1), async {
        while updates.status().phase == UpdatePhase::Checking {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(updates.status().phase, UpdatePhase::Available);
    assert_eq!(server.requests().len(), 1);
    updates.shutdown();
    updates.clock.advance_wall(TTL);
    updates.check(true).await;
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn chunked_response_cannot_evade_the_actual_body_bound() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0u8; 4096];
        let _ = socket.read(&mut buffer).await;
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        let chunk = format!("10000\r\n{}\r\n", "x".repeat(65536));
        for _ in 0..5 {
            if socket.write_all(chunk.as_bytes()).await.is_err() {
                break;
            }
        }
        let _ = socket.write_all(b"0\r\n\r\n").await;
    });
    let updates = Arc::new(Updates {
        endpoint,
        ..Default::default()
    });
    assert_eq!(updates.check(false).await.phase, UpdatePhase::Unavailable);
    server.await.unwrap();
}

#[tokio::test]
async fn shutdown_cancels_a_pending_check_without_waiting_for_the_network() {
    let server = Server::new(vec![
        Reply::json(200, release("v9.0.0").to_string()).gated(Arc::new(tokio::sync::Notify::new())),
    ])
    .await;
    let updates = Arc::new(Updates {
        endpoint: server.base.clone(),
        ..Default::default()
    });
    let check = {
        let updates = updates.clone();
        tokio::spawn(async move { updates.check(false).await })
    };
    server.wait_for_requests(1).await;
    updates.shutdown();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), check)
            .await
            .unwrap()
            .unwrap()
            .phase,
        UpdatePhase::Unavailable
    );
    assert_eq!(server.requests().len(), 1);
}
