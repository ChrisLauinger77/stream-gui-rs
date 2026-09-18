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
    poll_gate: StdMutex<Option<Arc<RequestGate>>>,
    refresh_gate: StdMutex<Option<Arc<RequestGate>>>,
    validation_gate: StdMutex<Option<Arc<RequestGate>>>,
    begin_gate: StdMutex<Option<Arc<RequestGate>>>,
    fail_revoke: bool,
}

#[derive(Default)]
struct RequestGate {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

impl RequestGate {
    async fn wait(&self) {
        self.entered.notify_one();
        self.release.notified().await;
    }
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
        scopes: vec!["user:read:follows".into()],
        expires_in: 14400,
    }
}

impl TwitchApi for FakeApi {
    async fn begin(&self, _: &str) -> Result<DeviceGrant> {
        let gate = self.begin_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.wait().await;
        }
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
        let gate = self.poll_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.wait().await;
        }
        self.polls
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(PollResult::Pending))
    }
    async fn validate(&self, _: &str) -> Result<Validation> {
        *self.validate_count.lock().unwrap() += 1;
        let gate = self.validation_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.wait().await;
        }
        self.validations
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(validation()))
    }
    async fn refresh(&self, _: &str, token: &str) -> Result<Credentials> {
        self.refresh_inputs.lock().unwrap().push(token.to_owned());
        let gate = self.refresh_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.wait().await;
        }
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
    assert!(
        service
            .state
            .lock()
            .await
            .store
            .load()
            .await
            .unwrap()
            .is_none()
    );
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
        assert!(
            service
                .state
                .lock()
                .await
                .store
                .load()
                .await
                .unwrap()
                .is_none()
        );
        assert!(service.status().await.user.is_none());
    }
}

#[tokio::test(start_paused = true)]
async fn concurrent_refresh_uses_each_rotating_token_once() {
    let service = Arc::new(service());
    authorize(&service).await;
    service
        .api
        .refreshes
        .lock()
        .unwrap()
        .extend([Ok(credentials("second")), Ok(credentials("third"))]);
    let gate = Arc::new(RequestGate::default());
    *service.api.refresh_gate.lock().unwrap() = Some(gate.clone());
    let first = {
        let service = service.clone();
        tokio::spawn(async move { service.refresh().await })
    };
    gate.entered.notified().await;
    let second = service.refresh();
    tokio::pin!(second);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut second)
            .await
            .is_err()
    );
    assert_eq!(
        *service.api.refresh_inputs.lock().unwrap(),
        ["refresh-initial"]
    );
    gate.release.notify_one();
    first.await.unwrap().unwrap();
    second.await.unwrap();
    assert_eq!(
        *service.api.refresh_inputs.lock().unwrap(),
        ["refresh-initial"]
    );
    let state = service.state.lock().await;
    assert_eq!(
        state
            .store
            .load()
            .await
            .unwrap()
            .unwrap()
            .refresh_token
            .as_str(),
        "refresh-second"
    );
}

#[tokio::test(start_paused = true)]
async fn logout_waits_for_in_flight_refresh_then_clears_credentials() {
    let service = Arc::new(service());
    authorize(&service).await;
    let gate = Arc::new(RequestGate::default());
    *service.api.refresh_gate.lock().unwrap() = Some(gate.clone());
    let request = {
        let service = service.clone();
        tokio::spawn(async move { service.refresh().await })
    };
    gate.entered.notified().await;
    let logout = service.logout();
    tokio::pin!(logout);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut logout)
            .await
            .is_err()
    );
    gate.release.notify_one();
    request.await.unwrap().unwrap();
    logout.await.unwrap();
    service.tick().await.unwrap();
    assert_eq!(service.status().await.phase, AuthPhase::SignedOut);
    assert!(
        service
            .state
            .lock()
            .await
            .store
            .load()
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test(start_paused = true)]
async fn refresh_at_known_expiration() {
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
    tokio::time::advance(Duration::from_secs(301)).await;
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
            .await
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
    assert!(
        service
            .state
            .lock()
            .await
            .store
            .load()
            .await
            .unwrap()
            .is_none()
    );
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
    assert!(
        service
            .state
            .lock()
            .await
            .store
            .load()
            .await
            .unwrap()
            .is_none()
    );
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

#[tokio::test(start_paused = true)]
async fn cancellation_interrupts_start_or_poll_without_late_authentication() {
    for starting in [true, false] {
        let service = Arc::new(service());
        let gate = Arc::new(RequestGate::default());
        if starting {
            *service.api.begin_gate.lock().unwrap() = Some(gate.clone());
        } else {
            service.login().await.unwrap();
            *service.api.poll_gate.lock().unwrap() = Some(gate.clone());
            service
                .api
                .polls
                .lock()
                .unwrap()
                .push_back(Ok(PollResult::Authorized(credentials("late"))));
            tokio::time::advance(Duration::from_secs(5)).await;
        }
        let pending = {
            let service = service.clone();
            tokio::spawn(async move {
                if starting {
                    service.login().await.map(|_| ())
                } else {
                    service.tick().await
                }
            })
        };
        gate.entered.notified().await;
        assert_eq!(service.status().await.phase, AuthPhase::Authorizing);
        assert_eq!(service.cancel().await.unwrap().phase, AuthPhase::Cancelled);
        assert_eq!(
            pending.await.unwrap().unwrap_err().code,
            ErrorCode::Cancelled
        );
        gate.release.notify_one();
        service.tick().await.unwrap();
        assert!(
            service
                .state
                .lock()
                .await
                .store
                .load()
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(service.status().await.phase, AuthPhase::Cancelled);
    }
}
#[tokio::test(start_paused = true)]
async fn cancellation_during_grant_validation_deletes_issued_credentials() {
    let service = Arc::new(service());
    service.login().await.unwrap();
    service
        .api
        .polls
        .lock()
        .unwrap()
        .push_back(Ok(PollResult::Authorized(credentials("late"))));
    let gate = Arc::new(RequestGate::default());
    *service.api.validation_gate.lock().unwrap() = Some(gate.clone());
    tokio::time::advance(Duration::from_secs(5)).await;
    let poll = {
        let service = service.clone();
        tokio::spawn(async move { service.tick().await })
    };
    gate.entered.notified().await;
    let cancellation = service.cancel();
    tokio::pin!(cancellation);
    assert!(
        tokio::time::timeout(Duration::from_millis(1), &mut cancellation)
            .await
            .is_err()
    );
    gate.release.notify_one();
    poll.await.unwrap().unwrap();
    assert_eq!(cancellation.await.unwrap().phase, AuthPhase::Cancelled);
    assert!(
        service
            .state
            .lock()
            .await
            .store
            .load()
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test(start_paused = true)]
async fn concurrent_validation_is_coalesced_and_status_does_not_wait_for_http() {
    let service = Arc::new(service());
    authorize(&service).await;
    let gate = Arc::new(RequestGate::default());
    *service.api.validation_gate.lock().unwrap() = Some(gate.clone());
    let first = {
        let service = service.clone();
        tokio::spawn(async move { service.validate().await })
    };
    gate.entered.notified().await;
    let second = service.validate();
    tokio::pin!(second);
    assert!(
        tokio::time::timeout(Duration::from_millis(1), &mut second)
            .await
            .is_err()
    );
    assert_eq!(
        tokio::time::timeout(Duration::from_millis(1), service.status())
            .await
            .unwrap()
            .phase,
        AuthPhase::Authenticated
    );
    gate.release.notify_one();
    first.await.unwrap().unwrap();
    second.await.unwrap();
    assert_eq!(*service.api.validate_count.lock().unwrap(), 2); // login + one shared validation
}
#[tokio::test(start_paused = true)]
async fn missing_required_scopes_are_rejected_but_additional_scopes_are_allowed() {
    for scopes in [
        vec![],
        vec!["unrelated".into()],
        vec!["user:read:follows".into(), "user:read:email".into()],
    ] {
        let accepted = scopes.iter().any(|scope| scope == "user:read:follows");
        let service = service();
        authorize(&service).await;
        service
            .api
            .validations
            .lock()
            .unwrap()
            .push_back(Ok(Validation {
                scopes,
                ..validation()
            }));
        let result = service.validate().await;
        if accepted {
            assert_eq!(result.unwrap().phase, AuthPhase::Authenticated);
        } else {
            assert_eq!(result.unwrap_err().code, ErrorCode::Unauthorized);
            assert!(
                service
                    .state
                    .lock()
                    .await
                    .store
                    .load()
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    }
}
#[derive(Clone, Default)]
struct PersistentTestStore(Arc<StdMutex<Option<Credentials>>>);
impl CredentialStore for PersistentTestStore {
    fn load(&self) -> Result<Option<Credentials>> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn save(&mut self, value: Credentials) -> Result<()> {
        *self.0.lock().unwrap() = Some(value);
        Ok(())
    }
    fn clear(&mut self) -> Result<()> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
    fn name(&self) -> &'static str {
        "test persistence"
    }
}
#[tokio::test(start_paused = true)]
async fn credentials_restore_across_instances_and_rotated_pair_is_persisted() {
    let mut store = PersistentTestStore::default();
    store.save(credentials("restored")).unwrap();
    let service = AuthService::new(
        FakeApi::default(),
        Some("client".into()),
        Box::new(store.clone()),
    );
    assert_eq!(service.status().await.phase, AuthPhase::Restoring);
    service.tick().await.unwrap();
    assert_eq!(service.status().await.phase, AuthPhase::Authenticated);
    service.refresh().await.unwrap();
    drop(service);
    assert_eq!(
        store.load().unwrap().unwrap().refresh_token.as_str(),
        "refresh-rotated"
    );
    let restored = AuthService::new(
        FakeApi::default(),
        Some("client".into()),
        Box::new(store.clone()),
    );
    restored.tick().await.unwrap();
    assert_eq!(*restored.api.validate_count.lock().unwrap(), 1);
    restored.logout().await.unwrap();
    assert!(store.load().unwrap().is_none());
}
#[tokio::test(start_paused = true)]
async fn expired_restored_token_refreshes_once_and_revocation_deletes_it() {
    for revoked in [false, true] {
        let mut store = MemoryCredentialStore::default();
        store.save(credentials("restored")).unwrap();
        let api = FakeApi::default();
        api.validations
            .lock()
            .unwrap()
            .push_back(Err(AppError::new(ErrorCode::AuthInvalid, "invalid")));
        if revoked {
            api.refreshes
                .lock()
                .unwrap()
                .push_back(Err(AppError::new(ErrorCode::AuthInvalid, "revoked")));
        }
        let service = AuthService::new(api, Some("client".into()), Box::new(store));
        let result = service.tick().await;
        assert_eq!(
            *service.api.refresh_inputs.lock().unwrap(),
            ["refresh-restored"]
        );
        if revoked {
            assert!(result.is_err());
            assert!(
                service
                    .state
                    .lock()
                    .await
                    .store
                    .load()
                    .await
                    .unwrap()
                    .is_none()
            );
        } else {
            result.unwrap();
            assert_eq!(service.status().await.phase, AuthPhase::Authenticated);
        }
    }
}
#[tokio::test(start_paused = true)]
async fn temporary_restore_validation_failure_recovers_without_consuming_refresh_token() {
    let mut store = MemoryCredentialStore::default();
    store.save(credentials("restored")).unwrap();
    let api = FakeApi::default();
    api.validations
        .lock()
        .unwrap()
        .push_back(Err(AppError::new(ErrorCode::Network, "offline")));
    let service = AuthService::new(api, Some("client".into()), Box::new(store));
    assert!(service.tick().await.is_err());
    assert_eq!(service.status().await.phase, AuthPhase::Error);
    tokio::time::advance(Duration::from_secs(60)).await;
    service.tick().await.unwrap();
    assert_eq!(service.status().await.phase, AuthPhase::Authenticated);
    assert!(service.api.refresh_inputs.lock().unwrap().is_empty());
}
#[tokio::test(start_paused = true)]
async fn dropping_refresh_caller_does_not_interrupt_rotation_or_reuse_old_token() {
    let service = Arc::new(service());
    authorize(&service).await;
    let gate = Arc::new(RequestGate::default());
    *service.api.refresh_gate.lock().unwrap() = Some(gate.clone());
    let caller = {
        let service = service.clone();
        tokio::spawn(async move { service.refresh().await })
    };
    gate.entered.notified().await;
    // The old rotating token is already absent from persistent storage.
    // Status remains readable while the credential owner is awaiting HTTP.
    assert_eq!(service.status().await.phase, AuthPhase::Authenticated);
    caller.abort();
    gate.release.notify_one();
    // A queued validation waits for the service-owned refresh to finish.
    service.validate().await.unwrap();
    assert_eq!(
        *service.api.refresh_inputs.lock().unwrap(),
        ["refresh-initial"]
    );
    assert_eq!(
        service
            .state
            .lock()
            .await
            .store
            .load()
            .await
            .unwrap()
            .unwrap()
            .refresh_token
            .as_str(),
        "refresh-rotated"
    );
}
#[tokio::test(start_paused = true)]
async fn storage_failure_is_explicit_without_insecure_authentication() {
    let store = crate::credentials::UnavailableCredentialStore(AppError::new(
        ErrorCode::CredentialStore,
        "locked",
    ));
    let service = AuthService::new(FakeApi::default(), Some("client".into()), Box::new(store));
    assert_eq!(
        service.login().await.unwrap_err().code,
        ErrorCode::CredentialStore
    );
    assert_eq!(service.status().await.phase, AuthPhase::Error);
    assert!(service.status().await.user.is_none());
}

#[tokio::test(start_paused = true)]
async fn explicit_validation_restores_and_validates_only_once() {
    let mut store = MemoryCredentialStore::default();
    store.save(credentials("stored")).unwrap();
    let service = AuthService::new(FakeApi::default(), Some("client".into()), Box::new(store));
    assert_eq!(
        service.validate().await.unwrap().phase,
        AuthPhase::Authenticated
    );
    assert_eq!(*service.api.validate_count.lock().unwrap(), 1);
}

#[tokio::test(start_paused = true)]
async fn refresh_cannot_change_the_authenticated_account() {
    let service = service();
    authorize(&service).await;
    service
        .api
        .validations
        .lock()
        .unwrap()
        .push_back(Ok(Validation {
            user_id: "different-account".into(),
            ..validation()
        }));
    assert_eq!(
        service.refresh().await.unwrap_err().code,
        ErrorCode::AuthInvalid
    );
    assert!(service.status().await.user.is_none());
    assert!(
        service
            .state
            .lock()
            .await
            .store
            .load()
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test(start_paused = true)]
async fn shutdown_drains_rotation_and_rejects_new_auth_work() {
    let service = Arc::new(service());
    authorize(&service).await;
    let gate = Arc::new(RequestGate::default());
    *service.api.refresh_gate.lock().unwrap() = Some(gate.clone());
    let refresh = {
        let service = service.clone();
        tokio::spawn(async move { service.refresh().await })
    };
    gate.entered.notified().await;
    let shutdown = service.shutdown();
    tokio::pin!(shutdown);
    assert!(
        tokio::time::timeout(Duration::from_millis(1), &mut shutdown)
            .await
            .is_err()
    );
    assert_eq!(
        service.validate().await.unwrap_err().code,
        ErrorCode::Cancelled
    );
    gate.release.notify_one();
    refresh.await.unwrap().unwrap();
    shutdown.await;
    assert_eq!(
        service
            .state
            .lock()
            .await
            .store
            .load()
            .await
            .unwrap()
            .unwrap()
            .refresh_token
            .as_str(),
        "refresh-rotated"
    );
    assert_eq!(
        service.login().await.unwrap_err().code,
        ErrorCode::Cancelled
    );
}

struct DeleteFailureStore {
    inner: MemoryCredentialStore,
    fail: Arc<AtomicBool>,
}
impl CredentialStore for DeleteFailureStore {
    fn load(&self) -> Result<Option<Credentials>> {
        self.inner.load()
    }
    fn save(&mut self, credentials: Credentials) -> Result<()> {
        self.inner.save(credentials)
    }
    fn clear(&mut self) -> Result<()> {
        if self.fail.load(Ordering::SeqCst) {
            Err(AppError::new(
                ErrorCode::CredentialStore,
                "Cannot delete test entry",
            ))
        } else {
            self.inner.clear()
        }
    }
    fn name(&self) -> &'static str {
        "test store"
    }
}
#[tokio::test(start_paused = true)]
async fn deletion_failure_prevents_refresh_and_cannot_claim_successful_logout() {
    let fail = Arc::new(AtomicBool::new(false));
    let service = AuthService::new(
        FakeApi::default(),
        Some("client".into()),
        Box::new(DeleteFailureStore {
            inner: MemoryCredentialStore::default(),
            fail: fail.clone(),
        }),
    );
    authorize(&service).await;
    fail.store(true, Ordering::SeqCst);
    assert_eq!(
        service.refresh().await.unwrap_err().code,
        ErrorCode::CredentialStore
    );
    assert!(service.api.refresh_inputs.lock().unwrap().is_empty());
    assert_eq!(
        service.logout().await.unwrap_err().code,
        ErrorCode::CredentialStore
    );
    assert_eq!(service.status().await.phase, AuthPhase::Error);
    assert!(service.status().await.user.is_none());
    assert!(
        service
            .state
            .lock()
            .await
            .store
            .load()
            .await
            .unwrap()
            .is_some()
    );
    fail.store(false, Ordering::SeqCst);
    assert_eq!(service.logout().await.unwrap().phase, AuthPhase::SignedOut);
    assert!(
        service
            .state
            .lock()
            .await
            .store
            .load()
            .await
            .unwrap()
            .is_none()
    );
}

/// Models a native adapter that loses the deletion status, as the locked macOS
/// dependency does. No real OS credential entry is accessed.
struct UnverifiedDeleteCredential {
    inner: keyring::mock::MockCredential,
    unreadable_after_delete: bool,
    deleted: AtomicBool,
}
impl keyring::credential::CredentialApi for UnverifiedDeleteCredential {
    fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
        self.inner.set_secret(secret)
    }
    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        if self.unreadable_after_delete && self.deleted.load(Ordering::SeqCst) {
            return Err(keyring::Error::Invalid(
                "synthetic-secret".into(),
                "synthetic-secret".into(),
            ));
        }
        self.inner.get_secret()
    }
    fn delete_credential(&self) -> keyring::Result<()> {
        self.deleted.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[tokio::test(start_paused = true)]
async fn unverified_native_deletion_blocks_refresh_and_logout_without_leaking_secrets() {
    for unreadable_after_delete in [false, true] {
        let store = crate::credentials::PlatformCredentialStore::with_test_credential(Box::new(
            UnverifiedDeleteCredential {
                inner: keyring::mock::MockCredential::default(),
                unreadable_after_delete,
                deleted: AtomicBool::new(false),
            },
        ));
        let service = AuthService::new(FakeApi::default(), Some("client".into()), Box::new(store));
        authorize(&service).await;
        let refresh = service.refresh().await.unwrap_err();
        assert_eq!(refresh.code, ErrorCode::CredentialStore);
        assert!(service.api.refresh_inputs.lock().unwrap().is_empty());
        let logout = service.logout().await.unwrap_err();
        assert_eq!(logout.code, ErrorCode::CredentialStore);
        let status = service.status().await;
        assert_eq!(status.phase, AuthPhase::Error);
        assert!(status.user.is_none());
        for output in [
            format!("{refresh:?}"),
            format!("{logout:?}"),
            serde_json::to_string(&status).unwrap(),
        ] {
            for secret in ["access-initial", "refresh-initial", "synthetic-secret"] {
                assert!(!output.contains(secret));
            }
        }
        if !unreadable_after_delete {
            assert!(
                service
                    .state
                    .lock()
                    .await
                    .store
                    .load()
                    .await
                    .unwrap()
                    .is_some()
            );
        }
    }
}
