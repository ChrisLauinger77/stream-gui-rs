use super::*;
use crate::{
    config::SettingsStore,
    helix::HelixClient,
    monitor::{LiveNotification, Monitor, MonitorPhase, NotificationSink},
    test_http::{Reply, Server},
    twitch_http::TwitchHttp,
};

#[derive(Default)]
struct Sink(StdMutex<Vec<LiveNotification>>);
impl NotificationSink for Sink {
    fn deliver(&self, event: LiveNotification) -> Result<()> {
        self.0.lock().unwrap().push(event);
        Ok(())
    }
}
fn page(stream: Option<&str>) -> String {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/helix/stream.json")).unwrap();
    let data: Vec<_> = stream
        .into_iter()
        .map(|id| {
            let mut value = fixture["data"][0].clone();
            value["id"] = id.into();
            value
        })
        .collect();
    serde_json::json!({"data": data, "pagination": {}}).to_string()
}
async fn login(auth: &AuthService<FakeApi>, clock: &Clock, user: &str) {
    let mut identity = validation();
    identity.user_id = user.into();
    auth.api.validations.lock().unwrap().push_back(Ok(identity));
    auth.api
        .polls
        .lock()
        .unwrap()
        .push_back(Ok(PollResult::Authorized(credentials("monitor-test"))));
    auth.login().await.unwrap();
    clock.advance_wall(Duration::from_secs(5));
    auth.tick().await.unwrap();
}
async fn cycle(monitor: &Monitor, helix: &HelixClient<FakeApi>) {
    helix.clear_cache();
    for _ in 0..monitor.snapshot().retry_in_seconds + 2 {
        monitor.test_advance(1).await;
    }
}

#[tokio::test]
async fn transient_validation_failure_keeps_same_session_stream_history() {
    session_history(None).await;
}
#[tokio::test]
async fn logout_and_other_account_login_reset_monitor_history() {
    session_history(Some("456")).await;
}
#[tokio::test]
async fn logout_and_same_account_new_session_reset_monitor_history() {
    session_history(Some("123")).await;
}
async fn session_history(replacement: Option<&str>) {
    let server = Server::new(vec![
        Reply::json(200, page(None)),
        Reply::json(200, page(Some("100"))),
        Reply::json(200, page(None)),
        Reply::json(200, page(Some("100"))),
        Reply::json(200, page(Some("101"))),
    ])
    .await;
    let clock = Clock::for_test();
    let auth = Arc::new(AuthService::with_clock(
        FakeApi::default(),
        Some("client".into()),
        Box::<MemoryCredentialStore>::default(),
        clock.clone(),
    ));
    login(&auth, &clock, "123").await;
    let (old_id, old_cancel) = auth.monitor_session().unwrap();
    let helix = Arc::new(HelixClient::new(
        TwitchHttp::for_test(&server.base, Duration::from_secs(2)),
        auth.clone(),
    ));
    let root = tempfile::tempdir().unwrap();
    let settings = Arc::new(SettingsStore::open(root.path()).unwrap());
    let mut prefs = settings.snapshot();
    prefs.background.monitoring_enabled = true;
    prefs.background.notifications_enabled = true;
    settings.update(prefs).unwrap();
    let monitor = Arc::new(Monitor::for_test());
    let sink = Arc::new(Sink::default());
    monitor.set_sink(sink.clone());
    monitor.start(auth.clone(), helix.clone(), settings);
    monitor.test_advance(0).await;
    cycle(&monitor, &helix).await;
    assert_eq!(sink.0.lock().unwrap().len(), 1);
    if let Some(user) = replacement {
        auth.logout().await.unwrap();
        assert!(old_cancel.is_cancelled());
        assert!(sink.0.lock().unwrap()[0].cancelled());
        monitor.test_advance(0).await;
        assert_eq!(monitor.snapshot().phase, MonitorPhase::SignedOut);
        assert_eq!(monitor.snapshot().live_count, None);
        login(&auth, &clock, user).await;
        assert_ne!(auth.monitor_session().unwrap().0, old_id);
    } else {
        auth.api
            .validations
            .lock()
            .unwrap()
            .push_back(Err(error(ErrorCode::Network)));
        assert!(auth.validate().await.is_err());
        assert!(auth.monitor_session().is_none());
        assert!(!old_cancel.is_cancelled());
        monitor.test_advance(0).await;
        assert_eq!(monitor.snapshot().phase, MonitorPhase::SignedOut);
        auth.validate().await.unwrap();
        assert_eq!(auth.monitor_session().unwrap().0, old_id);
        assert!(!old_cancel.is_cancelled());
    }
    cycle(&monitor, &helix).await; // Recovery baseline temporarily omits stream 100.
    assert_eq!(monitor.snapshot().live_count, Some(0));
    assert_eq!(sink.0.lock().unwrap().len(), 1);
    cycle(&monitor, &helix).await; // Same stream ID reappears.
    let expected = if replacement.is_some() { 2 } else { 1 };
    assert_eq!(sink.0.lock().unwrap().len(), expected);
    cycle(&monitor, &helix).await; // A genuinely new stream session still notifies.
    assert_eq!(sink.0.lock().unwrap().len(), expected + 1);
    assert_eq!(
        sink.0.lock().unwrap().last().unwrap().auth_session_id,
        auth.monitor_session().unwrap().0.to_string()
    );
    monitor.shutdown().await;
    auth.shutdown().await;
}
