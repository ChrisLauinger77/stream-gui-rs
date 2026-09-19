//! Fixed, local-only acceptance target. Never enters the monitor or Helix.
use crate::{domain::background::DesktopAction, monitor::LiveNotification};
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub(super) fn check_permission(
    permission: crate::domain::background::NotificationPermission,
) -> crate::domain::Result<()> {
    use crate::domain::background::NotificationPermission;
    match permission {
        NotificationPermission::Granted | NotificationPermission::OsManaged => Ok(()),
        _ => Err(super::delivery_error()),
    }
}

const SESSION: &str = "notification-acceptance";
// Zero is deliberately invalid as a real broadcaster ID.
const CHANNEL: &str = "0";

#[derive(Default)]
pub(super) struct TestNotifications {
    cancellation: Mutex<CancellationToken>,
}
impl TestNotifications {
    pub fn event(&self) -> LiveNotification {
        let cancellation = self
            .cancellation
            .lock()
            .expect("test notification token poisoned")
            .clone();
        LiveNotification {
            auth_session_id: SESSION.into(),
            broadcaster_id: CHANNEL.into(),
            display_name: "TEST notification: Synthetic channel".into(),
            title: "Native notification acceptance test".into(),
            category: "Synthetic target — no Twitch data".into(),
            session_cancel: cancellation.clone(),
            monitor_cancel: cancellation,
        }
    }
    pub fn clear(&self) {
        let mut cancellation = self
            .cancellation
            .lock()
            .expect("test notification token poisoned");
        cancellation.cancel();
        *cancellation = CancellationToken::new();
    }
}

pub(crate) fn is_test_action(action: &DesktopAction) -> bool {
    matches!(action, DesktopAction::Channel { auth_session_id, broadcaster_id, .. }
        if auth_session_id == SESSION && broadcaster_id == CHANNEL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submission_requires_native_permission() {
        use crate::domain::background::NotificationPermission::*;
        for permission in [Unknown, NotRequested, Denied, Unavailable] {
            assert!(check_permission(permission).is_err());
        }
        for permission in [Granted, OsManaged] {
            assert!(check_permission(permission).is_ok());
        }
    }

    #[test]
    fn clearing_cancels_all_old_tests_without_poisoning_new_tests() {
        let tests = TestNotifications::default();
        let first = tests.event();
        let second = tests.event();
        assert!(!first.cancelled());
        tests.clear();
        assert!(first.cancelled() && second.cancelled());
        assert!(!tests.event().cancelled());
    }

    #[test]
    fn only_the_fixed_synthetic_target_bypasses_authentication() {
        for (session, channel, expected) in [
            (SESSION, CHANNEL, true),
            (SESSION, "123", false),
            ("1", CHANNEL, false),
        ] {
            assert_eq!(
                is_test_action(&DesktopAction::Channel {
                    id: "test-action".into(),
                    auth_session_id: session.into(),
                    broadcaster_id: channel.into(),
                    display_name: "Synthetic channel".into(),
                }),
                expected
            );
        }
    }
}
