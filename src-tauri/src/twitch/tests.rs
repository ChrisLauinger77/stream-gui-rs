use super::api::{DeviceGrant, Validation};
use super::*;
use crate::credentials::MemoryCredentialStore;
use std::{collections::VecDeque, sync::Mutex as StdMutex};

#[derive(Default)]
struct FakeApi {
    polls: StdMutex<VecDeque<Result<PollResult>>>,
    validations: StdMutex<VecDeque<Result<Validation>>>,
    refreshes: StdMutex<VecDeque<Result<Credentials>>>,
    refresh_inputs: StdMutex<Vec<String>>,
    validate_count: StdMutex<u32>,
    poll_count: StdMutex<u32>,
    fail_revoke: bool,
}

fn credentials(suffix: &str) -> Credentials {
    Credentials {
        access_token: Zeroizing::new(format!("access-{suffix}")),
        refresh_token: Zeroizing::new(format!("refresh-{suffix}")),
    }
}

fn validation() -> Validation {
    Validation {
        client_id: "client".into(),
        user_id: "123".into(),
        login: "example".into(),
        scopes: vec![],
        expires_in: 14400,
    }
}

impl TwitchApi for FakeApi {
    async fn begin(&self, _: &str) -> Result<DeviceGrant> {
        Ok(DeviceGrant {
            device_code: "private-device-code".into(),
            user_code: "USERCODE".into(),
            verification_uri: "https://www.twitch.tv/activate?public=true&device-code=USERCODE"
                .into(),
            expires_in: 1800,
            interval: 5,
        })
    }
    async fn poll(&self, _: &str, _: &str) -> Result<PollResult> {
        *self.poll_count.lock().unwrap() += 1;
        self.polls
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(PollResult::Pending))
    }
    async fn validate(&self, _: &str) -> Result<Validation> {
        *self.validate_count.lock().unwrap() += 1;
        self.validations
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(validation()))
    }
    async fn refresh(&self, _: &str, token: &str) -> Result<Credentials> {
        self.refresh_inputs.lock().unwrap().push(token.to_owned());
        self.refreshes
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(credentials("rotated")))
    }
    async fn revoke(&self, _: &str, _: &str) -> Result<()> {
        if self.fail_revoke {
            Err(AppError::new(ErrorCode::Network, "offline"))
        } else {
            Ok(())
        }
    }
}

fn service() -> AuthService<FakeApi> {
    AuthService::new(
        FakeApi::default(),
        Some("client".into()),
        Box::<MemoryCredentialStore>::default(),
    )
}

async fn authorize(service: &AuthService<FakeApi>) {
    service
        .api
        .polls
        .lock()
        .unwrap()
        .push_back(Ok(PollResult::Authorized(credentials("initial"))));
    service.login().await.unwrap();
    tokio::time::advance(Duration::from_secs(5)).await;
    service.tick().await.unwrap();
    assert_eq!(service.status().await.phase, AuthPhase::Authenticated);
}

#[tokio::test(start_paused = true)]
async fn unconfigured_login_stops_at_registration_boundary() {
    let service = AuthService::new(
        FakeApi::default(),
        None,
        Box::<MemoryCredentialStore>::default(),
    );
    assert_eq!(service.status().await.phase, AuthPhase::NotConfigured);
    assert_eq!(
        service.login().await.unwrap_err().code,
        ErrorCode::NotConfigured
    );
    service.tick().await.unwrap();
    assert_eq!(*service.api.poll_count.lock().unwrap(), 0);
}

#[tokio::test(start_paused = true)]
async fn pending_polling_slowdown_and_expiration() {
    let service = service();
    service.login().await.unwrap();
    service.tick().await.unwrap();
    assert_eq!(*service.api.poll_count.lock().unwrap(), 0);
    tokio::time::advance(Duration::from_secs(5)).await;
    service.tick().await.unwrap();
    assert_eq!(*service.api.poll_count.lock().unwrap(), 1);
    service
        .api
        .polls
        .lock()
        .unwrap()
        .push_back(Ok(PollResult::SlowDown));
    tokio::time::advance(Duration::from_secs(5)).await;
    service.tick().await.unwrap();
    tokio::time::advance(Duration::from_secs(5)).await;
    service.tick().await.unwrap();
    assert_eq!(*service.api.poll_count.lock().unwrap(), 2);
    tokio::time::advance(Duration::from_secs(5)).await;
    service.tick().await.unwrap();
    assert_eq!(*service.api.poll_count.lock().unwrap(), 3);
    tokio::time::advance(Duration::from_secs(1800)).await;
    assert_eq!(
        service.tick().await.unwrap_err().code,
        ErrorCode::AuthExpired
    );
    assert!(service.status().await.authorization.is_none());
}

#[tokio::test(start_paused = true)]
async fn tokens_and_device_secret_never_appear_in_status() {
    let service = service();
    let pending = service.login().await.unwrap();
    let serialized = serde_json::to_string(&pending).unwrap();
    assert!(serialized.contains("USERCODE"));
    assert!(!serialized.contains("private-device-code"));
    authorize(&service).await;
    let serialized = serde_json::to_string(&service.status().await).unwrap();
    assert!(!serialized.contains("access-initial"));
    assert!(!serialized.contains("refresh-initial"));
    assert!(serialized.contains("example"));
}

#[tokio::test(start_paused = true)]
async fn logout_cancels_pending_grant_and_is_repeatable() {
    let service = service();
    service.login().await.unwrap();
    service
        .api
        .polls
        .lock()
        .unwrap()
        .push_back(Ok(PollResult::Authorized(credentials("late"))));
    service.logout().await.unwrap();
    service.logout().await.unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;
    service.tick().await.unwrap();
    assert_eq!(*service.api.poll_count.lock().unwrap(), 0);
    assert_eq!(service.status().await.phase, AuthPhase::SignedOut);
    assert!(service.verification_uri().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn denial_ends_grant_without_storing_credentials() {
    let service = service();
    service.login().await.unwrap();
    service
        .api
        .polls
        .lock()
        .unwrap()
        .push_back(Err(AppError::new(ErrorCode::AuthDenied, "declined")));
    tokio::time::advance(Duration::from_secs(5)).await;
    assert_eq!(
        service.tick().await.unwrap_err().code,
        ErrorCode::AuthDenied
    );
    assert!(service.state.lock().await.store.load().unwrap().is_none());
    assert_eq!(service.status().await.phase, AuthPhase::Error);
}

#[tokio::test(start_paused = true)]
async fn startup_and_hourly_validation() {
    let mut store = MemoryCredentialStore::default();
    store.save(credentials("restored")).unwrap();
    let service = AuthService::new(FakeApi::default(), Some("client".into()), Box::new(store));
    service.tick().await.unwrap();
    assert_eq!(*service.api.validate_count.lock().unwrap(), 1);
    tokio::time::advance(Duration::from_secs(3599)).await;
    service.tick().await.unwrap();
    assert_eq!(*service.api.validate_count.lock().unwrap(), 1);
    tokio::time::advance(Duration::from_secs(1)).await;
    service.tick().await.unwrap();
    assert_eq!(*service.api.validate_count.lock().unwrap(), 2);
}

#[tokio::test(start_paused = true)]
async fn validation_rejection_or_mismatched_client_clears_session() {
    for mismatch in [true, false] {
        let service = service();
        authorize(&service).await;
        let response = if mismatch {
            Ok(Validation {
                client_id: "wrong".into(),
                ..validation()
            })
        } else {
            Err(AppError::new(ErrorCode::AuthInvalid, "revoked"))
        };
        service.api.validations.lock().unwrap().push_back(response);
        assert_eq!(
            service.validate().await.unwrap_err().code,
            ErrorCode::AuthInvalid
        );
        assert!(service.state.lock().await.store.load().unwrap().is_none());
        assert!(service.status().await.user.is_none());
    }
}

#[tokio::test(start_paused = true)]
async fn concurrent_refresh_uses_each_rotating_token_once() {
    let service = service();
    authorize(&service).await;
    service
        .api
        .refreshes
        .lock()
        .unwrap()
        .extend([Ok(credentials("second")), Ok(credentials("third"))]);
    let (first, second) = tokio::join!(service.refresh(), service.refresh());
    first.unwrap();
    second.unwrap();
    assert_eq!(
        *service.api.refresh_inputs.lock().unwrap(),
        ["refresh-initial", "refresh-second"]
    );
    let state = service.state.lock().await;
    assert_eq!(
        state.store.load().unwrap().unwrap().refresh_token.as_str(),
        "refresh-third"
    );
}

#[tokio::test(start_paused = true)]
async fn proactive_refresh_near_expiration() {
    let service = service();
    service
        .api
        .validations
        .lock()
        .unwrap()
        .push_back(Ok(Validation {
            expires_in: 301,
            ..validation()
        }));
    authorize(&service).await;
    tokio::time::advance(Duration::from_secs(1)).await;
    service.tick().await.unwrap();
    assert_eq!(
        *service.api.refresh_inputs.lock().unwrap(),
        ["refresh-initial"]
    );
}

#[tokio::test(start_paused = true)]
async fn rotated_credentials_survive_temporary_validation_failure() {
    let service = service();
    authorize(&service).await;
    service
        .api
        .validations
        .lock()
        .unwrap()
        .push_back(Err(AppError::new(ErrorCode::Network, "offline")));
    assert!(service.refresh().await.is_err());
    assert_eq!(
        service
            .state
            .lock()
            .await
            .store
            .load()
            .unwrap()
            .unwrap()
            .refresh_token
            .as_str(),
        "refresh-rotated"
    );
    assert_eq!(service.status().await.phase, AuthPhase::Error);
    tokio::time::advance(Duration::from_secs(60)).await;
    service.tick().await.unwrap();
    assert_eq!(service.status().await.phase, AuthPhase::Authenticated);
}

#[tokio::test(start_paused = true)]
async fn ambiguous_refresh_failure_requires_new_login() {
    let service = service();
    authorize(&service).await;
    service
        .api
        .refreshes
        .lock()
        .unwrap()
        .push_back(Err(AppError::new(ErrorCode::Network, "offline")));
    assert!(service.refresh().await.is_err());
    assert!(service.state.lock().await.store.load().unwrap().is_none());
    assert!(service.status().await.user.is_none());
}

#[tokio::test(start_paused = true)]
async fn logout_clears_locally_even_if_revoke_is_offline() {
    let api = FakeApi {
        fail_revoke: true,
        ..FakeApi::default()
    };
    let service = AuthService::new(
        api,
        Some("client".into()),
        Box::<MemoryCredentialStore>::default(),
    );
    authorize(&service).await;
    let status = service.logout().await.unwrap();
    assert_eq!(status.phase, AuthPhase::SignedOut);
    assert!(status.error.is_some());
    assert!(service.state.lock().await.store.load().unwrap().is_none());
}

#[test]
fn browser_links_are_restricted_to_twitch_activation() {
    assert!(valid_verification_uri(
        "https://www.twitch.tv/activate?public=true&device-code=CODE"
    ));
    for uri in [
        "https://evil.example/activate",
        "javascript:alert(1)",
        "http://twitch.tv/activate",
        "https://user@twitch.tv/activate",
        "https://twitch.tv/other",
    ] {
        assert!(!valid_verification_uri(uri));
    }
}
