use super::*;
use crate::{
    config::SettingsStore,
    monitor::{LiveNotification, Monitor, MonitorPhase, NotificationSink},
};

fn streams(ids: &[(&str, &str)], cursor: Option<&str>) -> String {
    let template: serde_json::Value = serde_json::from_str(STREAM).unwrap();
    let data: Vec<_> = ids
        .iter()
        .map(|(id, user)| {
            let mut stream = template["data"][0].clone();
            stream["id"] = (*id).into();
            stream["user_id"] = (*user).into();
            stream
        })
        .collect();
    serde_json::json!({"data": data, "pagination": {"cursor": cursor}}).to_string()
}
#[tokio::test]
async fn monitor_scan_deduplicates_pages_and_reuses_foreground_cache() {
    let server = Server::new(vec![
        Reply::json(200, streams(&[("100", "1")], Some("next"))),
        Reply::json(200, streams(&[("100", "1"), ("200", "2")], None)),
    ])
    .await;
    let (client, _) = client(&server, None).await;
    let id = client.auth.monitor_session().unwrap().0;
    let result = client
        .monitor_followed(id, CachePolicy::Fresh, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
    let page = client
        .browse_followed_streams(
            browse::BrowseRequest {
                session_id: id.to_string(),
                cursor: None,
                refresh: false,
            },
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.freshness, browse::DataFreshness::Cached);
    assert_eq!(server.requests().len(), 2);
    assert!(server.requests()[1].contains("after=next"));
}
#[tokio::test]
async fn repeated_cursor_partial_failure_and_malformed_identity_never_complete_scan() {
    for replies in [
        vec![
            Reply::json(200, streams(&[("100", "1")], Some("repeat"))),
            Reply::json(200, streams(&[("100", "1")], Some("repeat"))),
        ],
        vec![
            Reply::json(200, streams(&[("100", "1")], Some("next"))),
            Reply::json(429, "PRIVATE"),
        ],
        vec![Reply::json(200, streams(&[("100", "wrong/url")], None))],
    ] {
        let server = Server::new(replies).await;
        let (client, _) = client(&server, None).await;
        let id = client.auth.monitor_session().unwrap().0;
        let error = client
            .monitor_followed(id, CachePolicy::Fresh, &CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(
            error.code,
            ErrorCode::Incomplete | ErrorCode::RateLimited | ErrorCode::InvalidResponse
        ));
        assert!(!error.message.contains("PRIVATE"));
    }
}
#[tokio::test]
async fn logout_cancels_scan_between_pages_without_rebinding() {
    let gate = Arc::new(tokio::sync::Notify::new());
    let server = Server::new(vec![
        Reply::json(200, streams(&[("100", "1")], Some("next"))).gated(gate.clone()),
    ])
    .await;
    let (client, _) = client(&server, None).await;
    let client = Arc::new(client);
    let id = client.auth.monitor_session().unwrap().0;
    let pending = {
        let client = client.clone();
        tokio::spawn(async move {
            client
                .monitor_followed(id, CachePolicy::Fresh, &CancellationToken::new())
                .await
        })
    };
    server.wait_for_requests(1).await;
    client.auth.logout().await.unwrap();
    gate.notify_one();
    assert!(matches!(
        pending.await.unwrap().unwrap_err().code,
        ErrorCode::Unauthenticated | ErrorCode::Cancelled
    ));
    assert_eq!(server.requests().len(), 1);
    assert!(
        client
            .monitor_followed(id, CachePolicy::Fresh, &CancellationToken::new())
            .await
            .is_err()
    );
}
#[derive(Default)]
struct Sink(Mutex<Vec<LiveNotification>>);
impl NotificationSink for Sink {
    fn deliver(&self, event: LiveNotification) -> Result<()> {
        self.0.lock().unwrap().push(event);
        Ok(())
    }
}
async fn phase(monitor: &Monitor, expected: MonitorPhase) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while monitor.snapshot().phase != expected {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn owned_monitor_cancels_pause_rebaselines_resume_and_drains_shutdown() {
    let gate = Arc::new(tokio::sync::Notify::new());
    let server = Server::new(vec![
        Reply::json(200, streams(&[("100", "1")], None)).gated(gate.clone()),
        Reply::json(200, streams(&[("100", "1"), ("200", "2")], None)),
    ])
    .await;
    let (client, _) = client(&server, None).await;
    let client = Arc::new(client);
    let root = tempfile::tempdir().unwrap();
    let settings = Arc::new(SettingsStore::open(root.path()).unwrap());
    let mut prefs = settings.snapshot();
    prefs.background.monitoring_enabled = true;
    prefs.background.notifications_enabled = true;
    settings.update(prefs).unwrap();
    let monitor = Arc::new(Monitor::default());
    let sink = Arc::new(Sink::default());
    monitor.set_sink(sink.clone());
    monitor.start(client.auth.clone(), client.clone(), settings.clone());
    // Starting twice cannot create competing loops.
    monitor.start(client.auth.clone(), client.clone(), settings);
    server.wait_for_requests(1).await;
    monitor.pause(true);
    gate.notify_one();
    phase(&monitor, MonitorPhase::Paused).await;
    assert!(sink.0.lock().unwrap().is_empty());
    monitor.pause(false);
    server.wait_for_requests(2).await;
    phase(&monitor, MonitorPhase::Running).await;
    assert_eq!(monitor.snapshot().live_count, Some(2));
    assert!(sink.0.lock().unwrap().is_empty());
    client.auth.logout().await.unwrap();
    phase(&monitor, MonitorPhase::SignedOut).await;
    assert_eq!(monitor.snapshot().live_count, None);
    monitor.shutdown().await;
    assert_eq!(monitor.snapshot().phase, MonitorPhase::Stopped);
    assert_eq!(server.requests().len(), 2);
    assert!(!Monitor::default().snapshot().paused);
}

#[tokio::test]
async fn monitor_backoff_recovery_sleep_and_channel_filtering_use_fresh_baselines() {
    let server = Server::new(vec![
        Reply::json(200, streams(&[("100", "1")], None)),
        Reply::json(
            200,
            streams(&[("100", "1"), ("200", "2"), ("300", "3")], None),
        ),
        Reply::json(503, "private body"),
        Reply::json(503, "private body"),
        Reply::json(
            200,
            streams(&[("100", "1"), ("200", "2"), ("400", "4")], None),
        ),
        Reply::json(200, streams(&[("500", "5")], None)),
        Reply::json(200, streams(&[("600", "6")], None)),
    ])
    .await;
    let (client, _) = client(&server, None).await;
    let client = Arc::new(client);
    let root = tempfile::tempdir().unwrap();
    let settings = Arc::new(SettingsStore::open(root.path()).unwrap());
    let mut prefs = settings.snapshot();
    prefs.background.monitoring_enabled = true;
    prefs.background.notifications_enabled = true;
    settings.update(prefs).unwrap();
    settings
        .set_channel(crate::config::SaveChannelSettingsRequest {
            broadcaster_id: "3".into(),
            overrides: crate::config::ChannelOverrides {
                notifications: Some(false),
                ..Default::default()
            },
        })
        .unwrap();
    let monitor = Arc::new(Monitor::for_test());
    let sink = Arc::new(Sink::default());
    monitor.set_sink(sink.clone());
    monitor.start(client.auth.clone(), client.clone(), settings);
    phase(&monitor, MonitorPhase::Running).await;
    assert!(sink.0.lock().unwrap().is_empty());
    // Advance the monitor clock one heartbeat at a time; synchronize on completed
    // iterations rather than sleeping or depending on network scheduling speed.
    async fn cycle(monitor: &Monitor, client: &HelixClient<TestApi>) {
        client.clear_cache();
        let seconds = monitor.snapshot().retry_in_seconds + 2;
        for _ in 0..seconds {
            monitor.test_advance(1).await;
        }
    }
    cycle(&monitor, &client).await;
    assert_eq!(sink.0.lock().unwrap().len(), 1);
    assert_eq!(sink.0.lock().unwrap()[0].broadcaster_id, "2");
    cycle(&monitor, &client).await;
    assert_eq!(monitor.snapshot().phase, MonitorPhase::Recovering);
    assert_eq!(monitor.snapshot().error, Some(ErrorCode::TwitchServer));
    assert!(monitor.snapshot().stale);
    cycle(&monitor, &client).await;
    assert_eq!(monitor.snapshot().phase, MonitorPhase::Running);
    assert_eq!(sink.0.lock().unwrap().len(), 1); // Channel 4 joined during outage.
    client.clear_cache();
    monitor.test_advance(3600).await; // Resume with a quiet current baseline.
    assert_eq!(monitor.snapshot().live_count, Some(1));
    assert_eq!(sink.0.lock().unwrap().len(), 1);
    cycle(&monitor, &client).await;
    assert_eq!(sink.0.lock().unwrap().len(), 2);
    assert_eq!(sink.0.lock().unwrap()[1].broadcaster_id, "6");
    monitor.pause(true);
    assert!(
        sink.0
            .lock()
            .unwrap()
            .iter()
            .all(LiveNotification::cancelled)
    );
    monitor.shutdown().await;
}

#[tokio::test]
async fn sleep_during_inflight_scan_discards_response_and_recovers_quietly() {
    let gate = Arc::new(tokio::sync::Notify::new());
    let server = Server::new(vec![
        Reply::json(200, streams(&[("100", "1")], None)).gated(gate.clone()),
        Reply::json(200, streams(&[("200", "2")], None)),
    ])
    .await;
    let (client, _) = client(&server, None).await;
    let client = Arc::new(client);
    let root = tempfile::tempdir().unwrap();
    let settings = Arc::new(SettingsStore::open(root.path()).unwrap());
    let mut prefs = settings.snapshot();
    prefs.background.monitoring_enabled = true;
    prefs.background.notifications_enabled = true;
    settings.update(prefs).unwrap();
    let monitor = Arc::new(Monitor::for_test());
    let sink = Arc::new(Sink::default());
    monitor.set_sink(sink.clone());
    monitor.start(client.auth.clone(), client.clone(), settings);
    server.wait_for_requests(1).await;
    monitor.test_advance(3600).await;
    assert_eq!(monitor.snapshot().phase, MonitorPhase::Recovering);
    assert_eq!(monitor.snapshot().error, Some(ErrorCode::Timeout));
    assert_eq!(monitor.snapshot().live_count, None);
    gate.notify_one();
    let seconds = monitor.snapshot().retry_in_seconds + 2;
    for _ in 0..seconds {
        monitor.test_advance(1).await;
    }
    assert_eq!(monitor.snapshot().phase, MonitorPhase::Running);
    assert_eq!(monitor.snapshot().live_count, Some(1));
    assert!(sink.0.lock().unwrap().is_empty());
    assert_eq!(server.requests().len(), 2);
    monitor.shutdown().await;
}

#[tokio::test]
async fn response_before_suspend_heartbeat_is_discarded_before_transition_acceptance() {
    let gate = Arc::new(tokio::sync::Notify::new());
    let server = Server::new(vec![
        Reply::json(200, streams(&[("100", "1")], None)),
        Reply::json(200, streams(&[("100", "1"), ("200", "2")], None)).gated(gate.clone()),
        Reply::json(200, streams(&[("300", "3")], None)),
        Reply::json(200, streams(&[("300", "3"), ("400", "4")], None)),
    ])
    .await;
    let (client, _) = client(&server, None).await;
    let client = Arc::new(client);
    let root = tempfile::tempdir().unwrap();
    let settings = Arc::new(SettingsStore::open(root.path()).unwrap());
    let mut prefs = settings.snapshot();
    prefs.background.monitoring_enabled = true;
    prefs.background.notifications_enabled = true;
    settings.update(prefs).unwrap();
    let monitor = Arc::new(Monitor::for_test());
    let sink = Arc::new(Sink::default());
    monitor.set_sink(sink.clone());
    monitor.start(client.auth.clone(), client.clone(), settings);
    phase(&monitor, MonitorPhase::Running).await;
    client.clear_cache();
    let advancing = {
        let monitor = monitor.clone();
        tokio::spawn(async move { advance_cycle(&monitor).await })
    };
    server.wait_for_requests(2).await;
    // Freeze uptime but jump wall time, as on a machine whose monotonic timer
    // pauses during suspend. Keep yielding a ready task so Tokio cannot auto-
    // advance its frozen clock while the loopback response completes.
    tokio::time::pause();
    let uptime = tokio::time::Instant::now();
    monitor.test_elapse(3600);
    gate.notify_one();
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !advancing.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "response did not finish"
        );
        tokio::task::yield_now().await;
    }
    advancing.await.unwrap();
    assert_eq!(tokio::time::Instant::now(), uptime);
    assert_eq!(monitor.snapshot().phase, MonitorPhase::Recovering);
    assert_eq!(monitor.snapshot().error, Some(ErrorCode::Timeout));
    assert!(sink.0.lock().unwrap().is_empty());
    tokio::time::resume();
    advance_cycle(&monitor).await;
    assert_eq!(monitor.snapshot().live_count, Some(1));
    assert!(sink.0.lock().unwrap().is_empty());
    client.clear_cache();
    advance_cycle(&monitor).await;
    assert_eq!(sink.0.lock().unwrap().len(), 1);
    assert_eq!(sink.0.lock().unwrap()[0].broadcaster_id, "4");
    monitor.shutdown().await;
}

async fn advance_cycle(monitor: &Monitor) {
    let seconds = monitor.snapshot().retry_in_seconds + 2;
    for _ in 0..seconds {
        monitor.test_advance(1).await;
    }
}

#[tokio::test]
async fn resume_baseline_bypasses_pre_pause_cache_and_ordinary_scans_reuse_it() {
    warm_cache_baseline(false).await;
}
#[tokio::test]
async fn suspend_baseline_bypasses_warm_cache_without_global_invalidation() {
    warm_cache_baseline(true).await;
}
async fn warm_cache_baseline(suspend: bool) {
    let server = Server::new(vec![
        Reply::json(200, streams(&[("100", "1")], None)),
        Reply::json(200, streams(&[("100", "1"), ("200", "2")], None)),
        Reply::json(
            200,
            streams(&[("100", "1"), ("200", "2"), ("300", "3")], None),
        ),
    ])
    .await;
    let (client, _) = client(&server, None).await;
    let client = Arc::new(client);
    let root = tempfile::tempdir().unwrap();
    let settings = Arc::new(SettingsStore::open(root.path()).unwrap());
    let mut prefs = settings.snapshot();
    prefs.background.monitoring_enabled = true;
    prefs.background.notifications_enabled = true;
    settings.update(prefs).unwrap();
    let monitor = Arc::new(Monitor::for_test());
    let sink = Arc::new(Sink::default());
    monitor.set_sink(sink.clone());
    monitor.start(client.auth.clone(), client.clone(), settings);
    phase(&monitor, MonitorPhase::Running).await;
    let id = client.auth.monitor_session().unwrap().0;
    let cached = client
        .monitor_followed(id, CachePolicy::Fresh, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(cached.len(), 1);
    assert_eq!(server.requests().len(), 1);
    if suspend {
        // Only the monitor clock jumps: the pre-suspend cache deliberately stays
        // warm, proving baseline policy rather than relying on TTL expiry.
        monitor.test_advance(3600).await;
    } else {
        monitor.pause(true);
        monitor.test_advance(0).await;
        monitor.pause(false);
        monitor.test_advance(0).await;
    }
    assert_eq!(server.requests().len(), 2);
    assert_eq!(monitor.snapshot().live_count, Some(2));
    assert!(sink.0.lock().unwrap().is_empty());
    let request = browse::BrowseRequest {
        session_id: id.to_string(),
        cursor: None,
        refresh: false,
    };
    let page = client
        .browse_followed_streams(request.clone(), &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(page.freshness, browse::DataFreshness::Cached);
    assert_eq!(page.items.len(), 2);
    // Foreground refresh observes a genuinely later transition; the ordinary
    // monitor scan may reuse it without another request or a baseline reset.
    client
        .browse_followed_streams(
            browse::BrowseRequest {
                refresh: true,
                ..request
            },
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    advance_cycle(&monitor).await;
    assert_eq!(server.requests().len(), 3);
    assert_eq!(sink.0.lock().unwrap().len(), 1);
    assert_eq!(sink.0.lock().unwrap()[0].broadcaster_id, "3");
    monitor.shutdown().await;
}
