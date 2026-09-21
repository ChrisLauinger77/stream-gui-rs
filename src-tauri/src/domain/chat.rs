//! Restricted browser/Chatterino chat: no frontend URL or executable crosses this boundary.
use super::{AppError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatRequest {
    pub auth_session_id: String,
    pub broadcaster_id: String,
}

#[derive(Clone, Debug)]
pub struct ChatTarget {
    url: String,
    login: String,
}
impl ChatTarget {
    pub(crate) fn for_login(login: &str) -> Result<Self> {
        crate::streamlink::playback::channel_url(login)?;
        Ok(Self {
            url: format!(
                "https://www.twitch.tv/popout/{}/chat",
                login.to_ascii_lowercase()
            ),
            login: login.to_ascii_lowercase(),
        })
    }
    pub fn url(&self) -> &str {
        &self.url
    }
}

/// Native composition supplies the opener; normal tests use a recording adapter.
pub trait ChatOpener: Send + Sync {
    fn open(&self, target: &ChatTarget) -> Result<()>;
}
struct Unavailable;
impl ChatOpener for Unavailable {
    fn open(&self, _: &ChatTarget) -> Result<()> {
        Err(open_error())
    }
}
pub fn open_error() -> AppError {
    AppError::new(
        ErrorCode::BrowserOpen,
        "Could not open Twitch chat in the default browser.",
    )
}

pub struct BrowserChat {
    opener: Arc<dyn ChatOpener>,
    chatterino: crate::chatterino::Chatterino,
    operation: Arc<Mutex<()>>,
    closing: Arc<AtomicBool>,
    slots: Arc<Semaphore>,
}
impl Default for BrowserChat {
    fn default() -> Self {
        Self::new(Arc::new(Unavailable))
    }
}
impl BrowserChat {
    pub fn new(opener: Arc<dyn ChatOpener>) -> Self {
        Self {
            opener,
            chatterino: Default::default(),
            operation: Arc::new(Mutex::new(())),
            closing: Arc::new(AtomicBool::new(false)),
            slots: Arc::new(Semaphore::new(4)),
        }
    }
    pub async fn open(&self, target: ChatTarget, cancel: CancellationToken) -> Result<()> {
        self.open_configured(target, crate::config::ChatProvider::Browser, None, cancel)
            .await
    }
    pub async fn open_configured(
        &self,
        target: ChatTarget,
        provider: crate::config::ChatProvider,
        path: Option<String>,
        cancel: CancellationToken,
    ) -> Result<()> {
        let permit = self.slots.clone().try_acquire_owned().map_err(|_| {
            AppError::new(
                ErrorCode::Capacity,
                "Browser chat is busy. Try again shortly.",
            )
        })?;
        let operation = self.operation.clone();
        let closing = self.closing.clone();
        let opener = self.opener.clone();
        let chatterino = self.chatterino.clone();
        // The task owns the blocking native call even if the IPC caller disappears.
        tokio::spawn(async move {
            let _permit = permit;
            let guard = operation.lock_owned().await;
            tokio::task::spawn_blocking(move || {
                let _guard = guard;
                if closing.load(Ordering::SeqCst) || cancel.is_cancelled() {
                    return Err(AppError::new(
                        ErrorCode::Cancelled,
                        "Browser chat request was cancelled.",
                    ));
                }
                match provider {
                    crate::config::ChatProvider::Browser => {
                        opener.open(&target).map_err(|_| open_error())
                    }
                    crate::config::ChatProvider::Chatterino => {
                        chatterino.open(path.as_deref(), &target.login)
                    }
                }
            })
            .await
            .map_err(|_| open_error())?
        })
        .await
        .map_err(|_| open_error())?
    }
    pub async fn shutdown(&self) {
        self.closing.store(true, Ordering::SeqCst);
        // Wait for the native opener already in progress; queued calls see closing.
        let _guard = self.operation.lock().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Recorder(std::sync::Mutex<Vec<String>>);
    impl ChatOpener for Recorder {
        fn open(&self, target: &ChatTarget) -> Result<()> {
            self.0.lock().unwrap().push(target.url().into());
            Ok(())
        }
    }
    #[test]
    fn chat_targets_accept_only_channel_logins_under_the_fixed_twitch_origin() {
        assert_eq!(
            ChatTarget::for_login("Example_1").unwrap().url(),
            "https://www.twitch.tv/popout/example_1/chat"
        );
        for input in [
            "",
            "https://evil.example",
            "//evil.example",
            "user/../../",
            "user?token=x",
            "user#fragment",
            "user@evil",
            "a b",
            "a\nb",
            "用户",
            "abcdefghijklmnopqrstuvwxyz",
        ] {
            assert!(ChatTarget::for_login(input).is_err());
        }
        assert!(
            serde_json::from_str::<ChatRequest>(
                r#"{"authSessionId":"1","broadcasterId":"123","url":"https://evil.example"}"#
            )
            .is_err()
        );
    }
    #[tokio::test]
    async fn manual_open_uses_the_injected_adapter_and_cancelled_requests_do_not_open() {
        let recorder = Arc::new(Recorder::default());
        let chat = BrowserChat::new(recorder.clone());
        chat.open(
            ChatTarget::for_login("example").unwrap(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            chat.open(ChatTarget::for_login("other").unwrap(), cancel)
                .await
                .unwrap_err()
                .code,
            ErrorCode::Cancelled
        );
        assert_eq!(
            *recorder.0.lock().unwrap(),
            ["https://www.twitch.tv/popout/example/chat"]
        );
        chat.shutdown().await;
        assert!(
            chat.open(
                ChatTarget::for_login("other").unwrap(),
                CancellationToken::new()
            )
            .await
            .is_err()
        );
        assert_eq!(recorder.0.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn queued_open_rechecks_auth_cancellation_before_native_dispatch() {
        let recorder = Arc::new(Recorder::default());
        let chat = Arc::new(BrowserChat::new(recorder.clone()));
        let guard = chat.operation.lock().await;
        let cancel = CancellationToken::new();
        let request = {
            let chat = chat.clone();
            let cancel = cancel.clone();
            tokio::spawn(async move {
                chat.open(ChatTarget::for_login("example").unwrap(), cancel)
                    .await
            })
        };
        while chat.slots.available_permits() == 4 {
            tokio::task::yield_now().await;
        }
        cancel.cancel();
        drop(guard);
        assert_eq!(
            request.await.unwrap().unwrap_err().code,
            ErrorCode::Cancelled
        );
        assert!(recorder.0.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn native_errors_are_categorized_without_echoing_adapter_details() {
        struct Failing;
        impl ChatOpener for Failing {
            fn open(&self, _: &ChatTarget) -> Result<()> {
                Err(AppError::new(ErrorCode::Internal, "untrusted native error"))
            }
        }
        let chat = BrowserChat::new(Arc::new(Failing));
        let error = chat
            .open(
                ChatTarget::for_login("example").unwrap(),
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::BrowserOpen);
        assert!(!error.message.contains("untrusted"));
    }
}
