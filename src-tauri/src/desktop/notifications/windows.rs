use super::*;
use ::windows::{
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    UI::Notifications::{
        NotificationSetting, ToastNotification, ToastNotificationManager, ToastNotifier,
    },
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    core::HSTRING,
};
use std::sync::atomic::{AtomicBool, Ordering};

struct Record {
    toast: ToastNotification,
    activation: i64,
    dismissal: i64,
    failure: i64,
    finished: Arc<AtomicBool>,
    event: LiveNotification,
    sent: Instant,
}
pub(super) struct Worker {
    shared: Arc<Shared>,
    notifier: Option<ToastNotifier>,
    records: Vec<Record>,
    initialized: bool,
}
impl Worker {
    pub fn new(shared: Arc<Shared>) -> Self {
        // This dedicated thread owns a balanced WinRT apartment and all native
        // handles. Callbacks only enqueue safe app actions on Tauri's main thread.
        let initialized = unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.is_ok();
        let notifier = initialized
            .then(|| {
                ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
                    "io.github.stream-gui-rs",
                ))
                .ok()
            })
            .flatten();
        let mut worker = Self {
            shared,
            notifier,
            records: vec![],
            initialized,
        };
        worker.refresh(false);
        worker
    }
    pub fn refresh(&mut self, _: bool) {
        let permission = match self.notifier.as_ref().and_then(|n| n.Setting().ok()) {
            Some(NotificationSetting::Enabled) => NotificationPermission::Granted,
            Some(
                NotificationSetting::DisabledForApplication
                | NotificationSetting::DisabledForUser
                | NotificationSetting::DisabledByGroupPolicy,
            ) => NotificationPermission::Denied,
            _ => NotificationPermission::Unavailable,
        };
        self.shared.permission(permission, true);
    }
    pub fn deliver(&mut self, event: LiveNotification) -> Result<()> {
        self.tick();
        if self.records.len() >= 32 {
            return Err(delivery_error());
        }
        let notifier = self.notifier.as_ref().ok_or_else(delivery_error)?;
        let document = XmlDocument::new().map_err(|_| delivery_error())?;
        document.LoadXml(&HSTRING::from(format!("<toast><visual><binding template=\"ToastGeneric\"><text>{} is live</text><text>{}</text><text>{}</text></binding></visual></toast>",
            escaped(&event.display_name), escaped(&event.title), escaped(&event.category)))).map_err(|_| delivery_error())?;
        let toast =
            ToastNotification::CreateToastNotification(&document).map_err(|_| delivery_error())?;
        let finished = Arc::new(AtomicBool::new(false));
        let activated = finished.clone();
        let clicked = event.clone();
        let shared = self.shared.clone();
        let activation = toast
            .Activated(&TypedEventHandler::new(move |_, _| {
                if !activated.swap(true, Ordering::SeqCst) {
                    shared.activate(&clicked);
                }
                Ok(())
            }))
            .map_err(|_| delivery_error())?;
        let dismissed = finished.clone();
        let dismissal = match toast.Dismissed(&TypedEventHandler::new(move |_, _| {
            dismissed.store(true, Ordering::SeqCst);
            Ok(())
        })) {
            Ok(token) => token,
            Err(_) => {
                let _ = toast.RemoveActivated(activation);
                return Err(delivery_error());
            }
        };
        let failed = finished.clone();
        let shared = self.shared.clone();
        let failure = match toast.Failed(&TypedEventHandler::new(move |_, _| {
            failed.store(true, Ordering::SeqCst);
            shared.failed();
            Ok(())
        })) {
            Ok(token) => token,
            Err(_) => {
                let _ = toast.RemoveActivated(activation);
                let _ = toast.RemoveDismissed(dismissal);
                return Err(delivery_error());
            }
        };
        if event.cancelled() || self.shared.stop.is_cancelled() {
            let _ = toast.RemoveActivated(activation);
            let _ = toast.RemoveDismissed(dismissal);
            let _ = toast.RemoveFailed(failure);
            return Ok(());
        }
        if notifier.Show(&toast).is_err() {
            let _ = toast.RemoveActivated(activation);
            let _ = toast.RemoveDismissed(dismissal);
            let _ = toast.RemoveFailed(failure);
            return Err(delivery_error());
        }
        self.records.push(Record {
            toast,
            activation,
            dismissal,
            failure,
            finished,
            event,
            sent: Instant::now(),
        });
        Ok(())
    }
    pub fn tick(&mut self) {
        self.records.retain(|record| {
            if record.finished.load(Ordering::SeqCst)
                || record.event.cancelled()
                || self.shared.stop.is_cancelled()
                || record.sent.elapsed() > Duration::from_secs(900)
            {
                let _ = record.toast.RemoveActivated(record.activation);
                let _ = record.toast.RemoveDismissed(record.dismissal);
                let _ = record.toast.RemoveFailed(record.failure);
                if let Some(notifier) = &self.notifier {
                    let _ = notifier.Hide(&record.toast);
                }
                false
            } else {
                true
            }
        });
    }
    pub fn shutdown(&mut self) {
        self.tick();
        self.notifier = None;
        if self.initialized {
            // Matched only to a successful RoInitialize on this same worker.
            unsafe {
                RoUninitialize();
            }
        }
    }
}
