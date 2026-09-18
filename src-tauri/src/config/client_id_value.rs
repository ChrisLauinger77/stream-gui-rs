use std::ffi::OsStr;

pub const BUILD_ENV: &str = "TWITCH_CLIENT_ID_BUILD";

// Shared by build.rs and the Rust runtime. This is a public identifier, not a
// credential. Do not assume a fixed provider length or echo rejected input.
pub fn validate(value: &OsStr) -> Result<&str, &'static str> {
    value
        .to_str()
        .filter(|id| {
            !id.is_empty() && id.len() <= 128 && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
        .ok_or("must contain 1 to 128 ASCII letters or digits, without whitespace")
}
