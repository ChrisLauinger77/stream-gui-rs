use super::client_id_value::{BUILD_ENV, validate};
use crate::domain::{AppError, ErrorCode, Result};
use std::ffi::OsStr;

const RUNTIME_ENV: &str = "TWITCH_CLIENT_ID";

#[cfg(feature = "desktop")]
pub(crate) fn from_environment() -> Result<String> {
    let runtime = std::env::var_os(RUNTIME_ENV);
    // build.rs always emits this value; an empty value marks an unconfigured
    // development build. Packaged/release builds cannot reach that state.
    let embedded = env!("STREAM_GUI_RS_EMBEDDED_TWITCH_CLIENT_ID");
    resolve(
        runtime.as_deref(),
        (!embedded.is_empty()).then_some(embedded),
    )
}

fn resolve(runtime: Option<&OsStr>, embedded: Option<&str>) -> Result<String> {
    let (value, source) = match (runtime, embedded) {
        // An explicitly set but invalid override is an error, never a silent
        // fallback to a different OAuth application and credential-store key.
        (Some(value), _) => (value, RUNTIME_ENV),
        (None, Some(value)) => (OsStr::new(value), BUILD_ENV),
        (None, None) => {
            return Err(AppError::new(
                ErrorCode::NotConfigured,
                "Twitch client ID is missing. Build with TWITCH_CLIENT_ID_BUILD, or set TWITCH_CLIENT_ID for development and restart. No client secret is needed.",
            ));
        }
    };
    validate(value).map(str::to_owned).map_err(|reason| {
        AppError::new(
            ErrorCode::NotConfigured,
            format!(
                "{source} {reason}. Supply a public Twitch client ID; no client secret is needed."
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMBEDDED: &str = "embeddedPublicClient123";
    const OVERRIDE: &str = "developerPublicClient456";

    #[test]
    fn embedded_id_needs_no_runtime_environment() {
        assert_eq!(resolve(None, Some(EMBEDDED)).unwrap(), EMBEDDED);
    }

    #[test]
    fn developer_override_takes_precedence_and_works_without_an_embedded_id() {
        for embedded in [Some(EMBEDDED), None] {
            assert_eq!(
                resolve(Some(OsStr::new(OVERRIDE)), embedded).unwrap(),
                OVERRIDE
            );
        }
    }

    #[test]
    fn missing_both_is_an_actionable_configuration_error() {
        let error = resolve(None, None).unwrap_err();
        assert_eq!(error.code, ErrorCode::NotConfigured);
        assert!(error.message.contains(BUILD_ENV));
        assert!(error.message.contains(RUNTIME_ENV));
    }

    #[test]
    fn invalid_values_are_rejected_without_fallback_or_echoing_input() {
        for value in [
            "",
            " ",
            " public123",
            "public123 ",
            "public id",
            "public-id",
            "id_123",
            "id\n123",
            "id\r123",
            "id\t123",
            "id\x00123",
            "püblic123",
            &"a".repeat(129),
        ] {
            for (runtime, embedded, source) in [
                (Some(OsStr::new(value)), Some(EMBEDDED), RUNTIME_ENV),
                (None, Some(value), BUILD_ENV),
            ] {
                let error = resolve(runtime, embedded).unwrap_err();
                assert_eq!(error.code, ErrorCode::NotConfigured);
                assert!(error.message.starts_with(source));
                assert!(error.message.contains("ASCII letters or digits"));
            }
        }
        let error =
            resolve(Some(OsStr::new("do-not-echo-this-value")), Some(EMBEDDED)).unwrap_err();
        assert!(!error.message.contains("do-not-echo-this-value"));
    }

    #[test]
    fn identifiers_are_case_preserving_and_bounded_without_a_fixed_length() {
        for value in ["A", "abc123XYZ", &"a".repeat(128)] {
            assert_eq!(resolve(None, Some(value)).unwrap(), value);
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_override_is_rejected_instead_of_ignored() {
        use std::os::unix::ffi::OsStrExt;
        let error = resolve(Some(OsStr::from_bytes(b"invalid\xff")), Some(EMBEDDED)).unwrap_err();
        assert!(error.message.starts_with(RUNTIME_ENV));
    }

    #[cfg(windows)]
    #[test]
    fn non_unicode_override_is_rejected_instead_of_ignored() {
        use std::{ffi::OsString, os::windows::ffi::OsStringExt};
        let value = OsString::from_wide(&[0xd800]);
        let error = resolve(Some(&value), Some(EMBEDDED)).unwrap_err();
        assert!(error.message.starts_with(RUNTIME_ENV));
    }
}
