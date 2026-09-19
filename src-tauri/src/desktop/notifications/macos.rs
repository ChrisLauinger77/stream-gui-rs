use super::*;
use block2::{DynBlock, RcBlock};
use objc2::{
    AnyThread, DefinedClass, define_class,
    rc::Retained,
    runtime::{Bool, ProtocolObject},
};
use objc2_foundation::{NSArray, NSBundle, NSError, NSObject, NSObjectProtocol, NSString};
use objc2_user_notifications::*;
use std::{
    collections::HashMap,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};

type Records = Arc<Mutex<HashMap<String, (LiveNotification, Instant)>>>;
struct DelegateState {
    shared: Arc<Shared>,
    records: Records,
}
define_class!(
    #[unsafe(super(NSObject))]
    #[name = "StreamGuiNotificationDelegate"]
    #[ivars = DelegateState]
    struct Delegate;
    // SAFETY: NSObject's inherited implementation satisfies this marker protocol.
    unsafe impl NSObjectProtocol for Delegate {}
    // SAFETY: method signatures match UNUserNotificationCenterDelegate. Completion
    // blocks are invoked once during each callback; borrowed native values do not escape.
    unsafe impl UNUserNotificationCenterDelegate for Delegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn present(
            &self,
            _center: &UNUserNotificationCenter,
            notification: &UNNotification,
            completion: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            let valid = self
                .ivars()
                .records
                .lock()
                .expect("notification registry poisoned")
                .get(&notification.request().identifier().to_string())
                .is_some_and(|(event, _)| {
                    !event.cancelled() && !self.ivars().shared.stop.is_cancelled()
                });
            completion.call((if valid {
                UNNotificationPresentationOptions::Banner | UNNotificationPresentationOptions::List
            } else {
                UNNotificationPresentationOptions::empty()
            },));
        }
        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn respond(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion: &DynBlock<dyn Fn()>,
        ) {
            let id = response.notification().request().identifier().to_string();
            let event = self
                .ivars()
                .records
                .lock()
                .expect("notification registry poisoned")
                .remove(&id);
            // Only Apple's default click action selects the internally retained target.
            if response.actionIdentifier().to_string()
                == "com.apple.UNNotificationDefaultActionIdentifier"
            {
                if let Some((event, _)) = event {
                    self.ivars().shared.activate(&event);
                }
            }
            completion.call(());
        }
    }
);
impl Delegate {
    fn new(shared: Arc<Shared>, records: Records) -> Retained<Self> {
        let object = Self::alloc().set_ivars(DelegateState { shared, records });
        // SAFETY: NSObject's designated initializer, on our newly allocated object.
        unsafe { objc2::msg_send![super(object), init] }
    }
}
pub(super) struct Worker {
    shared: Arc<Shared>,
    center: Option<Retained<UNUserNotificationCenter>>,
    _delegate: Option<Retained<Delegate>>,
    records: Records,
    requested: bool,
    checking: Arc<AtomicBool>,
}
impl Worker {
    pub fn new(shared: Arc<Shared>) -> Self {
        let records: Records = Arc::default();
        // UNUserNotificationCenter can raise an exception outside an app bundle.
        let center = NSBundle::mainBundle()
            .bundleIdentifier()
            .filter(|id| id.to_string() == "io.github.stream-gui-rs")
            .map(|_| UNUserNotificationCenter::currentNotificationCenter());
        let delegate = center.as_ref().map(|center| {
            let delegate = Delegate::new(shared.clone(), records.clone());
            center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            delegate
        });
        let mut worker = Self {
            shared,
            center,
            _delegate: delegate,
            records,
            requested: false,
            checking: Arc::new(AtomicBool::new(false)),
        };
        worker.refresh(false);
        worker
    }
    pub fn refresh(&mut self, request: bool) {
        let Some(center) = &self.center else {
            self.shared
                .permission(NotificationPermission::Unavailable, false);
            return;
        };
        if request && !self.requested {
            self.requested = true;
            let shared = self.shared.clone();
            center.requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert,
                &RcBlock::new(move |allowed: Bool, error: *mut NSError| {
                    if !shared.stop.is_cancelled() {
                        shared.permission(
                            if !error.is_null() {
                                NotificationPermission::Unavailable
                            } else if allowed.as_bool() {
                                NotificationPermission::Granted
                            } else {
                                NotificationPermission::Denied
                            },
                            true,
                        );
                    }
                }),
            );
        }
        if self.checking.swap(true, Ordering::SeqCst) {
            return;
        }
        let checking = self.checking.clone();
        let shared = self.shared.clone();
        center.getNotificationSettingsWithCompletionHandler(&RcBlock::new(
            move |settings: NonNull<UNNotificationSettings>| {
                // SAFETY: Apple's non-null settings object is valid for this callback.
                let permission = match unsafe { settings.as_ref() }.authorizationStatus() {
                    UNAuthorizationStatus::NotDetermined => NotificationPermission::NotRequested,
                    UNAuthorizationStatus::Denied => NotificationPermission::Denied,
                    UNAuthorizationStatus::Authorized
                    | UNAuthorizationStatus::Provisional
                    | UNAuthorizationStatus::Ephemeral => NotificationPermission::Granted,
                    _ => NotificationPermission::Unknown,
                };
                if !shared.stop.is_cancelled() {
                    shared.permission(permission, true);
                }
                checking.store(false, Ordering::SeqCst);
            },
        ));
    }
    pub fn deliver(&mut self, event: LiveNotification) -> Result<()> {
        self.tick();
        let center = self.center.as_ref().ok_or_else(delivery_error)?;
        if self
            .shared
            .state
            .lock()
            .expect("native notification state poisoned")
            .permission
            != NotificationPermission::Granted
            || self
                .records
                .lock()
                .expect("notification registry poisoned")
                .len()
                >= 32
        {
            return Err(delivery_error());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&format!(
            "{} is live",
            event.display_name
        )));
        content.setSubtitle(&NSString::from_str(&event.category));
        content.setBody(&NSString::from_str(&event.title));
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&id),
            &content,
            None,
        );
        if event.cancelled() || self.shared.stop.is_cancelled() {
            return Ok(());
        }
        self.records
            .lock()
            .expect("notification registry poisoned")
            .insert(id.clone(), (event, Instant::now()));
        let records = self.records.clone();
        let shared = self.shared.clone();
        center.addNotificationRequest_withCompletionHandler(
            &request,
            Some(&RcBlock::new(move |error: *mut NSError| {
                if !error.is_null() {
                    records
                        .lock()
                        .expect("notification registry poisoned")
                        .remove(&id);
                    shared.failed();
                }
            })),
        );
        Ok(())
    }
    pub fn tick(&mut self) {
        let mut records = self.records.lock().expect("notification registry poisoned");
        let mut expired = Vec::new();
        records.retain(|id, (event, sent)| {
            let remove = event.cancelled()
                || self.shared.stop.is_cancelled()
                || sent.elapsed() > Duration::from_secs(900);
            if remove {
                expired.push(NSString::from_str(id));
            }
            !remove
        });
        drop(records);
        if let Some(center) = &self.center {
            if !expired.is_empty() {
                let ids = NSArray::from_retained_slice(&expired);
                center.removePendingNotificationRequestsWithIdentifiers(&ids);
                center.removeDeliveredNotificationsWithIdentifiers(&ids);
            }
        }
    }
    pub fn shutdown(&mut self) {
        self.tick();
        if let Some(center) = &self.center {
            center.setDelegate(None);
        }
    }
}
