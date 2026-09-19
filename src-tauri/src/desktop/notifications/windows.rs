use super::windows_state::{APP_ID, CAPACITY, RETENTION, Targets};
use super::*;
use ::windows::{
    Data::Xml::Dom::XmlDocument,
    Foundation::{DateTime, PropertyValue, TypedEventHandler},
    UI::Notifications::{
        NotificationSetting, ToastDismissalReason, ToastDismissedEventArgs, ToastNotification,
        ToastNotificationManager, ToastNotifier,
    },
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    core::{HSTRING, Interface},
};
use std::collections::HashMap;
#[path = "windows_activation.rs"]
mod activation;

struct Record {
    toast: ToastNotification,
    activation: i64,
    dismissal: i64,
    failure: i64,
}
pub(super) struct Worker {
    shared: Arc<Shared>,
    notifier: Option<ToastNotifier>,
    records: HashMap<String, Record>,
    targets: Arc<Mutex<Targets>>,
    registration: Option<activation::Registration>,
    initialized: bool,
}
impl Worker {
    pub fn new(shared: Arc<Shared>) -> Self {
        // This dedicated thread owns a balanced WinRT apartment and all native
        // handles. Callbacks only enqueue safe app actions on Tauri's main thread.
        let initialized = unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.is_ok();
        let targets = Arc::new(Mutex::new(Targets::default()));
        let registration = initialized
            .then(|| activation::Registration::new(shared.clone(), targets.clone()).ok())
            .flatten();
        let notifier = registration.as_ref().and_then(|_| {
            ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID)).ok()
        });
        // No target survives application termination; remove orphaned history
        // from a prior crash before accepting any new activation IDs.
        if let Some(history) = notifier
            .as_ref()
            .and_then(|_| ToastNotificationManager::History().ok())
        {
            let _ = history.ClearWithId(&HSTRING::from(APP_ID));
        }
        let mut worker = Self {
            shared,
            notifier,
            records: HashMap::new(),
            targets,
            registration,
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
        self.shared.permission(permission, self.notifier.is_some());
    }
    pub fn deliver(&mut self, event: LiveNotification) -> Result<()> {
        self.tick();
        if self.shared.stop.is_cancelled() || event.cancelled() {
            return Ok(());
        }
        if self.records.len() >= CAPACITY {
            return Err(delivery_error());
        }
        let id = self
            .targets
            .lock()
            .expect("notification targets poisoned")
            .insert(event.clone())
            .ok_or_else(delivery_error)?;
        let result = self.show(&id, &event);
        if result.is_err() {
            self.targets
                .lock()
                .expect("notification targets poisoned")
                .retire(&id);
            self.tick();
        }
        result
    }
    fn show(&mut self, id: &str, event: &LiveNotification) -> Result<()> {
        let notifier = self.notifier.as_ref().ok_or_else(delivery_error)?;
        let document = XmlDocument::new().map_err(|_| delivery_error())?;
        document.LoadXml(&HSTRING::from(format!("<toast launch=\"{id}\"><visual><binding template=\"ToastGeneric\"><text>{} is live</text><text>{}</text><text>{}</text></binding></visual></toast>", escaped(&event.display_name), escaped(&event.title), escaped(&event.category)))).map_err(|_| delivery_error())?;
        let toast =
            ToastNotification::CreateToastNotification(&document).map_err(|_| delivery_error())?;
        // Two 16-character fields retain all 128 random bits and satisfy WinRT's
        // tag/group limits, while allowing exact Notification Center removal.
        toast
            .SetTag(&HSTRING::from(&id[..16]))
            .map_err(|_| delivery_error())?;
        toast
            .SetGroup(&HSTRING::from(&id[16..]))
            .map_err(|_| delivery_error())?;
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            + RETENTION;
        let expiration = DateTime {
            UniversalTime: ((unix.as_secs() + 11_644_473_600) * 10_000_000) as i64,
        };
        let expiration = PropertyValue::CreateDateTime(expiration)
            .and_then(|v| v.cast::<::windows::Foundation::IReference<DateTime>>())
            .map_err(|_| delivery_error())?;
        toast
            .SetExpirationTime(&expiration)
            .map_err(|_| delivery_error())?;
        let clicked = id.to_owned();
        let targets = self.targets.clone();
        let shared = self.shared.clone();
        let activation = toast
            .Activated(&TypedEventHandler::new(move |_, _| {
                activation::activate(&shared, &targets, APP_ID, &clicked);
                Ok(())
            }))
            .map_err(|_| delivery_error())?;
        let dismissed = id.to_owned();
        let targets = self.targets.clone();
        let dismissal = match toast.Dismissed(&TypedEventHandler::<
            ToastNotification,
            ToastDismissedEventArgs,
        >::new(move |_, args| {
            if let Some(reason) = args.as_ref().and_then(|args| args.Reason().ok()) {
                targets
                    .lock()
                    .expect("notification targets poisoned")
                    .dismissed(&dismissed, reason == ToastDismissalReason::TimedOut);
            }
            Ok(())
        })) {
            Ok(token) => token,
            Err(_) => {
                let _ = toast.RemoveActivated(activation);
                return Err(delivery_error());
            }
        };
        let failed = id.to_owned();
        let targets = self.targets.clone();
        let shared = self.shared.clone();
        let failure = match toast.Failed(&TypedEventHandler::new(move |_, _| {
            targets
                .lock()
                .expect("notification targets poisoned")
                .retire(&failed);
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
        self.records.insert(
            id.into(),
            Record {
                toast: toast.clone(),
                activation,
                dismissal,
                failure,
            },
        );
        if event.cancelled() || self.shared.stop.is_cancelled() {
            self.targets
                .lock()
                .expect("notification targets poisoned")
                .retire(id);
            self.tick();
            return Ok(());
        }
        notifier.Show(&toast).map_err(|_| delivery_error())
    }
    pub fn tick(&mut self) {
        let retired = self
            .targets
            .lock()
            .expect("notification targets poisoned")
            .collect_retired(self.shared.stop.is_cancelled());
        for id in retired {
            if let Some(record) = self.records.remove(&id) {
                let _ = record.toast.RemoveActivated(record.activation);
                let _ = record.toast.RemoveDismissed(record.dismissal);
                let _ = record.toast.RemoveFailed(record.failure);
                if let Some(notifier) = &self.notifier {
                    let _ = notifier.Hide(&record.toast);
                }
                if let Ok(history) = ToastNotificationManager::History() {
                    let _ = history.RemoveGroupedTagWithId(
                        &HSTRING::from(&id[..16]),
                        &HSTRING::from(&id[16..]),
                        &HSTRING::from(APP_ID),
                    );
                }
            }
        }
    }
    pub fn shutdown(&mut self) {
        self.shared.stop.cancel();
        self.tick();
        self.registration = None;
        self.notifier = None;
        if self.initialized {
            // Matched only to a successful RoInitialize on this same worker.
            unsafe {
                RoUninitialize();
            }
        }
    }
}
