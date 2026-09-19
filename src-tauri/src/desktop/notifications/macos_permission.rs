//! Serialize authorization and settings callbacks so polling cannot hide a failed request.
use crate::domain::background::NotificationPermission;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Operation {
    Settings,
    Request,
}

#[derive(Default)]
pub(super) struct PermissionCheck {
    active: Option<Operation>,
    request_pending: bool,
    request_failed: bool,
}
impl PermissionCheck {
    pub fn begin(&mut self, request: bool) -> Option<Operation> {
        // Coalesce repeated clicks while authorization is already in progress.
        if request && self.active != Some(Operation::Request) {
            self.request_pending = true;
        }
        if self.active.is_some() {
            return None;
        }
        let operation = if std::mem::take(&mut self.request_pending) {
            Operation::Request
        } else {
            Operation::Settings
        };
        self.active = Some(operation);
        Some(operation)
    }

    pub fn request_pending(&self) -> bool {
        self.request_pending
    }

    pub fn complete(&mut self, permission: NotificationPermission) -> NotificationPermission {
        if self.active.take() == Some(Operation::Request) {
            self.request_failed = permission == NotificationPermission::Unavailable;
        }
        match permission {
            NotificationPermission::NotRequested if self.request_failed => {
                // macOS may still report NotDetermined after rejecting the application.
                NotificationPermission::Unavailable
            }
            NotificationPermission::Granted | NotificationPermission::Denied => {
                self.request_failed = false;
                permission
            }
            _ => permission,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use NotificationPermission::*;

    #[test]
    fn request_waits_for_old_settings_callback_and_blocks_overlapping_polls() {
        let mut check = PermissionCheck::default();
        assert_eq!(check.begin(false), Some(Operation::Settings));
        assert_eq!(check.begin(true), None);
        assert!(check.request_pending());
        assert_eq!(check.complete(NotRequested), NotRequested);
        assert_eq!(check.begin(false), Some(Operation::Request));
        assert_eq!(check.begin(false), None);
        assert_eq!(check.begin(true), None);
        assert_eq!(check.complete(Granted), Granted);
        assert!(!check.request_pending());
        assert_eq!(check.begin(false), Some(Operation::Settings));
    }

    #[test]
    fn failed_authorization_survives_polling_and_can_be_retried() {
        let mut check = PermissionCheck::default();
        assert_eq!(check.begin(true), Some(Operation::Request));
        assert_eq!(check.complete(Unavailable), Unavailable);
        assert_eq!(check.begin(false), Some(Operation::Settings));
        assert_eq!(check.complete(NotRequested), Unavailable);
        assert_eq!(check.begin(true), Some(Operation::Request));
        assert_eq!(check.complete(Granted), Granted);
        assert_eq!(check.begin(false), Some(Operation::Settings));
        assert_eq!(check.complete(NotRequested), NotRequested);
    }

    #[test]
    fn system_permission_changes_recover_from_a_request_error() {
        for permission in [Granted, Denied] {
            let mut check = PermissionCheck::default();
            check.begin(true);
            check.complete(Unavailable);
            check.begin(false);
            assert_eq!(check.complete(permission), permission);
        }
    }
}
