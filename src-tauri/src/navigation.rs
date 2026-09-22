//! External navigation has no execution, credential or playback authority.
use crate::domain::{AppError, ErrorCode, Result};
use serde::Serialize;
use std::sync::Mutex;
use ts_rs::TS;

pub const MAX_LINK_BYTES: usize = 256;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum NavigationIntent {
    Show,
    Channel { login: String },
    Category { id: String },
    Team { name: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PendingNavigation {
    pub id: String,
    pub intent: NavigationIntent,
}

pub fn parse_link(input: &str) -> Result<NavigationIntent> {
    let invalid = || {
        AppError::new(
            ErrorCode::InvalidInput,
            "Invalid application navigation link.",
        )
    };
    // Validate raw spelling before URL normalization can erase dot segments,
    // whitespace, encoded delimiters or credentials. This grammar needs no escapes.
    if input.len() > MAX_LINK_BYTES
        || !input.is_ascii()
        || input
            .bytes()
            .any(|c| c.is_ascii_control() || c.is_ascii_whitespace())
    {
        return Err(invalid());
    }
    let rest = input.strip_prefix("stream-gui-rs://").ok_or_else(invalid)?;
    if rest == "show" {
        return Ok(NavigationIntent::Show);
    }
    let (action, value) = rest.split_once('/').ok_or_else(invalid)?;
    match action {
        "channel"
            if !value.is_empty()
                && value.len() <= 25
                && value
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_') =>
        {
            Ok(NavigationIntent::Channel {
                login: value.to_ascii_lowercase(),
            })
        }
        "category" => {
            crate::config::validate_broadcaster_id(value).map_err(|_| invalid())?;
            Ok(NavigationIntent::Category { id: value.into() })
        }
        "team" => Ok(NavigationIntent::Team {
            name: crate::helix::browse::normalize_team_name(value).map_err(|_| invalid())?,
        }),
        _ => Err(invalid()),
    }
}
/// Only the executable and at most one OS protocol argument are accepted.
/// An ordinary launch restores the existing window without creating navigation.
pub fn parse_arguments(args: &[String]) -> Result<Option<NavigationIntent>> {
    match args {
        [_executable] => Ok(None),
        [_executable, link] => parse_link(link).map(Some),
        _ => Err(AppError::new(
            ErrorCode::InvalidInput,
            "Expected one application navigation link.",
        )),
    }
}
#[derive(Default)]
pub struct NavigationInbox {
    pending: Mutex<Option<PendingNavigation>>,
}
impl NavigationInbox {
    pub fn receive(&self, intent: NavigationIntent) {
        *self.pending.lock().expect("navigation mutex poisoned") = Some(PendingNavigation {
            id: uuid::Uuid::new_v4().to_string(),
            intent,
        });
    }
    pub fn snapshot(&self) -> Option<PendingNavigation> {
        self.pending
            .lock()
            .expect("navigation mutex poisoned")
            .clone()
    }
    pub fn acknowledge(&self, id: &str) {
        let mut pending = self.pending.lock().expect("navigation mutex poisoned");
        if pending.as_ref().is_some_and(|p| p.id == id) {
            *pending = None;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_have_only_typed_navigation_authority() {
        assert_eq!(
            parse_link("stream-gui-rs://show").unwrap(),
            NavigationIntent::Show
        );
        assert_eq!(
            parse_link("stream-gui-rs://channel/Example_1").unwrap(),
            NavigationIntent::Channel {
                login: "example_1".into()
            }
        );
        assert_eq!(
            parse_link("stream-gui-rs://category/123").unwrap(),
            NavigationIntent::Category { id: "123".into() }
        );
        assert_eq!(
            parse_link("stream-gui-rs://team/Example-Team").unwrap(),
            NavigationIntent::Team {
                name: "example-team".into()
            }
        );
    }
    #[test]
    fn hostile_or_ambiguous_links_are_rejected_without_echoing_input() {
        for input in [
            "",
            "https://channel/a",
            "STREAM-GUI-RS://show",
            "stream-gui-rs://show/",
            "stream-gui-rs://show?autoplay=1",
            "stream-gui-rs://watch/a",
            "stream-gui-rs://channel/a/b",
            "stream-gui-rs://channel/a/../b",
            "stream-gui-rs://channel/é",
            "stream-gui-rs://channel/%61",
            "stream-gui-rs://channel/a%2fb",
            "stream-gui-rs://channel/a%5cb",
            "stream-gui-rs://channel/a%252fb",
            "stream-gui-rs://channel/a?token=private",
            "stream-gui-rs://channel/a#x",
            "stream-gui-rs://channel/a\\b",
            "stream-gui-rs://channel/",
            "stream-gui-rs://user:secret@channel/a",
            "stream-gui-rs://channel:80/a",
            "stream-gui-rs://category/0",
            "stream-gui-rs://category/01",
            "stream-gui-rs://category/name",
            "stream-gui-rs://team/../x",
            " stream-gui-rs://show",
            "stream-gui-rs://channel/a\n",
            "stream-gui-rs://channel/a\0",
            "stream-gui-rs://settings/execute",
        ] {
            let error = parse_link(input).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidInput);
            assert_eq!(error.message, "Invalid application navigation link.");
        }
        assert!(parse_link(&format!("stream-gui-rs://team/{}", "x".repeat(10000))).is_err());
    }
    #[test]
    fn pending_delivery_is_latest_only_and_old_ack_cannot_remove_new_intent() {
        let inbox = NavigationInbox::default();
        assert!(inbox.snapshot().is_none());
        inbox.receive(NavigationIntent::Show);
        let old = inbox.snapshot().unwrap();
        for _ in 0..1000 {
            inbox.receive(NavigationIntent::Channel {
                login: "synthetic".into(),
            });
        }
        let latest = inbox.snapshot().unwrap();
        assert_ne!(old.id, latest.id);
        inbox.acknowledge(&old.id);
        assert_eq!(inbox.snapshot(), Some(latest.clone()));
        inbox.acknowledge(&latest.id);
        assert!(inbox.snapshot().is_none());
    }
    #[test]
    fn arguments_cannot_carry_a_second_execution_surface() {
        assert!(parse_arguments(&["app".into()]).unwrap().is_none());
        assert_eq!(
            parse_arguments(&["app".into(), "stream-gui-rs://show".into()]).unwrap(),
            Some(NavigationIntent::Show)
        );
        for args in [
            vec![],
            vec!["app", "show"],
            vec!["app", "--player", "sh"],
            vec!["app", "stream-gui-rs://show", "--anything"],
        ] {
            assert!(
                parse_arguments(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err()
            );
        }
    }
}
