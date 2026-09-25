use super::localization;
use crate::{
    domain::{
        AppError, ErrorCode, Result,
        background::{DesktopAction, NotificationPermission},
    },
    monitor::{LiveNotification, NotificationSink},
};
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, SyncSender},
    },
    time::{Duration, Instant},
};
use tauri::Manager;
use tokio_util::sync::CancellationToken;

#[cfg(feature = "notification-acceptance")]
pub(super) mod acceptance;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::Worker;
#[cfg(windows)]
mod windows;
#[cfg(any(windows, test))]
mod windows_state;
#[cfg(windows)]
use windows::Worker;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "macos", test))]
mod macos_permission;
#[cfg(target_os = "macos")]
use macos::Worker;

pub(super) struct NativeState {
    pub permission: NotificationPermission,
    pub clicks: bool,
    pub tray_host: bool,
    pub action: Option<DesktopAction>,
}
pub(super) struct Shared {
    pub state: Mutex<NativeState>,
    pub stop: CancellationToken,
    pub app: tauri::AppHandle,
}
impl Shared {
    fn locale(&self) -> localization::Locale {
        localization::current(&self.app)
    }
    fn failed(&self) {
        if let Some(services) = self
            .app
            .try_state::<Arc<crate::domain::services::Services>>()
        {
            services.monitor.notification_failed();
        }
    }
    fn permission(&self, value: NotificationPermission, clicks: bool) {
        let mut state = self
            .state
            .lock()
            .expect("native notification state poisoned");
        state.permission = value;
        state.clicks = clicks;
    }
    fn activate(&self, event: &LiveNotification) {
        if event.cancelled() || self.stop.is_cancelled() {
            return;
        }
        let event = event.clone();
        let app = self.app.clone();
        let stop = self.stop.clone();
        let _ = self.app.run_on_main_thread(move || {
            let notifications = app.state::<Arc<Notifications>>();
            let mut state = notifications
                .shared
                .state
                .lock()
                .expect("native notification state poisoned");
            // Serialize the final cancellation check with clearing a pending action.
            if event.cancelled() || stop.is_cancelled() {
                return;
            }
            state.action = Some(DesktopAction::Channel {
                id: uuid::Uuid::new_v4().to_string(),
                auth_session_id: event.auth_session_id,
                broadcaster_id: event.broadcaster_id,
                display_name: event.display_name,
            });
            drop(state);
            super::show_window(&app);
        });
    }
}
enum Command {
    Live(LiveNotification),
    Permission,
}
pub(super) struct Notifications {
    pub shared: Arc<Shared>,
    sender: SyncSender<Command>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    #[cfg(feature = "notification-acceptance")]
    tests: acceptance::TestNotifications,
}
impl Notifications {
    pub fn new(app: tauri::AppHandle) -> Arc<Self> {
        let (sender, receiver) = mpsc::sync_channel(32);
        let shared = Arc::new(Shared {
            state: Mutex::new(NativeState {
                permission: NotificationPermission::Unknown,
                clicks: false,
                tray_host: !cfg!(target_os = "linux"),
                action: None,
            }),
            stop: CancellationToken::new(),
            app,
        });
        let owner = shared.clone();
        let thread = std::thread::spawn(move || {
            let mut worker = native_scope(|| Worker::new(owner.clone()));
            let mut check = Instant::now();
            while !owner.stop.is_cancelled() {
                let connected = native_scope(|| {
                    if check.elapsed() >= Duration::from_secs(5) {
                        worker.refresh(false);
                        check = Instant::now();
                    }
                    worker.tick();
                    match receiver.recv_timeout(Duration::from_millis(100)) {
                        Ok(Command::Live(event)) if !event.cancelled() => {
                            let delivered = worker.deliver(event);
                            if delivered.is_err() {
                                owner.failed();
                            }
                        }
                        Ok(Command::Permission) => worker.refresh(true),
                        Err(mpsc::RecvTimeoutError::Disconnected) => return false,
                        _ => {}
                    }
                    true
                });
                if !connected {
                    break;
                }
            }
            native_scope(|| {
                worker.shutdown();
                drop(worker);
            });
        });
        Arc::new(Self {
            shared,
            sender,
            thread: Mutex::new(Some(thread)),
            #[cfg(feature = "notification-acceptance")]
            tests: acceptance::TestNotifications::default(),
        })
    }
    pub fn request_permission(&self) -> Result<()> {
        self.sender
            .try_send(Command::Permission)
            .map_err(|_| delivery_error())
    }
    #[cfg(feature = "notification-acceptance")]
    pub fn test_notification(
        &self,
        request: crate::domain::background::NotificationTestAction,
    ) -> Result<()> {
        use crate::domain::background::NotificationTestAction;
        match request {
            NotificationTestAction::Send => {
                acceptance::check_permission(
                    self.shared
                        .state
                        .lock()
                        .expect("native notification state poisoned")
                        .permission,
                )?;
                self.deliver(self.tests.event())
            }
            NotificationTestAction::Clear => {
                self.tests.clear();
                let mut state = self
                    .shared
                    .state
                    .lock()
                    .expect("native notification state poisoned");
                if state
                    .action
                    .as_ref()
                    .is_some_and(acceptance::is_test_action)
                {
                    state.action = None;
                }
                Ok(())
            }
        }
    }
    pub async fn shutdown(&self) {
        self.shared.stop.cancel();
        let thread = self
            .thread
            .lock()
            .expect("notification thread poisoned")
            .take();
        if let Some(thread) = thread {
            let _ = tokio::task::spawn_blocking(move || thread.join()).await;
        }
        self.shared
            .state
            .lock()
            .expect("native notification state poisoned")
            .action = None;
    }
}
// Cocoa needs short-lived autorelease pools on a long-running non-main thread.
fn native_scope<T>(operation: impl FnOnce() -> T) -> T {
    #[cfg(target_os = "macos")]
    {
        objc2::rc::autoreleasepool(|_| operation())
    }
    #[cfg(not(target_os = "macos"))]
    {
        operation()
    }
}
impl NotificationSink for Notifications {
    fn deliver(&self, event: LiveNotification) -> Result<()> {
        if self.shared.stop.is_cancelled() || event.cancelled() {
            return Ok(());
        }
        self.sender
            .try_send(Command::Live(event))
            .map_err(|_| delivery_error())
    }
}
fn delivery_error() -> AppError {
    AppError::new(
        ErrorCode::Notification,
        "Desktop notification delivery is unavailable.",
    )
}
#[cfg(any(target_os = "linux", windows, test))]
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    #[test]
    fn provider_text_cannot_be_notification_markup() {
        assert_eq!(
            super::escaped("<a href=\"url\">A&B</a>"),
            "&lt;a href=&quot;url&quot;&gt;A&amp;B&lt;/a&gt;"
        );
    }
}
