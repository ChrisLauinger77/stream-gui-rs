//! Windows banner events and Notification Center activation share this bounded
//! registry. Native callbacks carry only an opaque ID, never a navigation target.
use crate::{monitor::LiveNotification, time::Clock};
use std::{collections::HashMap, time::Duration};

pub(super) const APP_ID: &str = "io.github.stream-gui-rs";
pub(super) const RETENTION: Duration = Duration::from_secs(900);
pub(super) const CAPACITY: usize = 32;
struct Target {
    event: LiveNotification,
    expires: Duration,
    retired: bool,
}
#[derive(Default)]
pub(super) struct Targets {
    entries: HashMap<String, Target>,
    clock: Clock,
}
impl Targets {
    pub fn insert(&mut self, event: LiveNotification) -> Option<String> {
        if self.entries.len() >= CAPACITY || event.cancelled() {
            return None;
        }
        let id = uuid::Uuid::new_v4().simple().to_string();
        self.entries.insert(
            id.clone(),
            Target {
                event,
                expires: self.clock.now() + RETENTION,
                retired: false,
            },
        );
        Some(id)
    }
    pub fn activate(&mut self, app_id: &str, id: &str, stopping: bool) -> Option<LiveNotification> {
        if stopping || app_id != APP_ID {
            return None;
        }
        let target = self.entries.get_mut(id)?;
        if target.retired || target.event.cancelled() || self.clock.now() >= target.expires {
            return None;
        }
        target.retired = true; // Banner and COM callbacks cannot navigate twice.
        Some(target.event.clone())
    }
    pub fn dismissed(&mut self, id: &str, timed_out: bool) {
        // TimedOut only moves the banner to Notification Center. UserCanceled
        // and ApplicationHidden retire the notification itself.
        if !timed_out {
            self.retire(id);
        }
    }
    pub fn retire(&mut self, id: &str) {
        if let Some(target) = self.entries.get_mut(id) {
            target.retired = true;
        }
    }
    pub fn collect_retired(&mut self, stopping: bool) -> Vec<String> {
        let now = self.clock.now();
        let mut retired = Vec::new();
        self.entries.retain(|id, target| {
            let remove =
                stopping || target.retired || target.event.cancelled() || now >= target.expires;
            if remove {
                retired.push(id.clone());
            }
            !remove
        });
        retired
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::sync::CancellationToken;
    #[cfg(feature = "notification-acceptance")]
    #[test]
    fn acceptance_notifications_use_normal_retention_and_clear_only_test_targets() {
        let tests = super::super::acceptance::TestNotifications::default();
        let mut targets = Targets::default();
        let real = targets.insert(event("123")).unwrap();
        let test = targets.insert(tests.event()).unwrap();
        targets.dismissed(&test, true);
        assert!(targets.collect_retired(false).is_empty());
        assert_eq!(
            targets
                .activate(APP_ID, &test, false)
                .unwrap()
                .broadcaster_id,
            "0"
        );
        assert_eq!(targets.collect_retired(false), [test]);
        let test = targets.insert(tests.event()).unwrap();
        tests.clear();
        assert!(targets.activate(APP_ID, &test, false).is_none());
        assert_eq!(targets.collect_retired(false), [test]);
        assert!(targets.activate(APP_ID, &real, false).is_some());
    }
    fn event(id: &str) -> LiveNotification {
        LiveNotification {
            auth_session_id: "1".into(),
            broadcaster_id: id.into(),
            display_name: "Channel".into(),
            title: "Title".into(),
            category: "Category".into(),
            session_cancel: CancellationToken::new(),
            monitor_cancel: CancellationToken::new(),
        }
    }
    #[test]
    fn banner_timeout_keeps_notification_center_target_and_activation_is_once() {
        let mut targets = Targets::default();
        let id = targets.insert(event("123")).unwrap();
        targets.dismissed(&id, true);
        assert!(targets.collect_retired(false).is_empty());
        assert_eq!(
            targets.activate(APP_ID, &id, false).unwrap().broadcaster_id,
            "123"
        );
        assert!(targets.activate(APP_ID, &id, false).is_none());
        assert_eq!(targets.collect_retired(false), [id]);
        assert!(targets.entries.is_empty());
    }
    #[test]
    fn explicit_retirement_and_user_dismissal_release_native_ids_once() {
        let mut targets = Targets::default();
        let id = targets.insert(event("123")).unwrap();
        targets.dismissed(&id, false);
        assert!(targets.activate(APP_ID, &id, false).is_none());
        assert_eq!(targets.collect_retired(false), [id]);
        let id = targets.insert(event("456")).unwrap();
        targets.retire(&id);
        assert_eq!(targets.collect_retired(false), [id]);
        assert!(targets.collect_retired(false).is_empty());
    }
    #[test]
    fn retention_is_bounded_and_expiration_includes_suspend() {
        let clock = Clock::for_test();
        let mut targets = Targets {
            clock: clock.clone(),
            ..Default::default()
        };
        let mut ids = Vec::new();
        for _ in 0..CAPACITY {
            ids.push(targets.insert(event("123")).unwrap());
        }
        assert!(targets.insert(event("456")).is_none());
        clock.advance_wall(RETENTION);
        assert!(targets.activate(APP_ID, &ids[0], false).is_none());
        assert_eq!(targets.collect_retired(false).len(), CAPACITY);
        assert!(targets.insert(event("456")).is_some());
    }
    #[test]
    fn old_account_unknown_payload_and_shutdown_cannot_activate() {
        let mut targets = Targets::default();
        let original = event("123");
        let id = targets.insert(original.clone()).unwrap();
        assert!(targets.activate("another.app", &id, false).is_none());
        assert!(
            targets
                .activate(APP_ID, "https://untrusted.invalid", false)
                .is_none()
        );
        original.session_cancel.cancel();
        assert!(targets.activate(APP_ID, &id, false).is_none());
        assert_eq!(targets.collect_retired(false), [id]);
        let id = targets.insert(event("456")).unwrap();
        assert!(targets.activate(APP_ID, &id, true).is_none());
        assert_eq!(targets.collect_retired(true), std::slice::from_ref(&id));
        assert!(targets.entries.is_empty());
        assert!(targets.activate(APP_ID, &id, false).is_none());
    }
}
