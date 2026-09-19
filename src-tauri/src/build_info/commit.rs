// Shared by the build script and deterministic unit tests. Never echo arbitrary
// environment values into Cargo directives or the application metadata.
pub fn short_commit(value: Option<&str>) -> String {
    match value {
        Some(value)
            if (7..=64).contains(&value.len())
                && value.bytes().all(|byte| byte.is_ascii_hexdigit()) =>
        {
            value[..7].to_ascii_lowercase()
        }
        _ => "unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_sha_is_shortened_to_seven_lowercase_characters() {
        assert_eq!(short_commit(Some(&"ABCDEF0123".repeat(4))), "abcdef0");
        assert_eq!(short_commit(Some(&"1234ABCD".repeat(8))), "1234abc");
    }

    #[test]
    fn short_sha_is_preserved() {
        assert_eq!(short_commit(Some("a1b2c3d")), "a1b2c3d");
    }

    #[test]
    fn missing_or_invalid_metadata_is_unknown() {
        for value in [
            None,
            Some(""),
            Some("main"),
            Some("123456"),
            Some("abcdefg"),
            Some("a1b2c3d-dirty"),
            Some("a1b2c3d\ncargo:rustc-env=INJECTED=yes"),
            Some("éabcdef"),
            Some(&"a".repeat(65)),
        ] {
            assert_eq!(short_commit(value), "unknown");
        }
    }
}
