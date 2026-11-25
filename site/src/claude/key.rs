//! The visitor's Anthropic API key, kept out of every printed form.

use std::fmt;

/// The prefix of every Anthropic API key.
const PREFIX: &str = "sk-ant-";

/// The longest key accepted, in bytes. Real keys are about a hundred.
const MAX_LEN: usize = 256;

/// An Anthropic API key a visitor entered.
///
/// `Debug` and `Display` print `[redacted]`, so the key cannot reach a log line, an
/// error message or a panic through formatting. It is not `Clone`, `Serialize` or
/// `Deserialize`: share it behind an `Arc`, and the only way to read it is
/// [`expose`](Self::expose), which the HTTP client calls to set the `x-api-key`
/// header. The bytes are overwritten when the key is dropped.
pub struct ApiKey(String);

/// Why a key was rejected before any request was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyError {
    Empty,
    /// It does not start with `sk-ant-`.
    Prefix,
    /// It is too long, or has spaces or other characters keys do not use.
    Shape,
}

impl KeyError {
    /// The message shown under the key field.
    pub fn message(self) -> &'static str {
        match self {
            Self::Empty => "Paste a key first.",
            Self::Prefix => "Anthropic API keys start with sk-ant-.",
            Self::Shape => "That does not look like an Anthropic API key.",
        }
    }
}

impl ApiKey {
    /// Checks the shape of a pasted key. Surrounding whitespace is ignored. This does
    /// not check that the key works; the first request does.
    pub fn parse(raw: &str) -> Result<Self, KeyError> {
        let key = raw.trim();
        if key.is_empty() {
            return Err(KeyError::Empty);
        }
        if !key.starts_with(PREFIX) {
            return Err(KeyError::Prefix);
        }
        let shaped = key.len() <= MAX_LEN
            && key.len() > PREFIX.len() + 8
            && key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
        if !shaped {
            return Err(KeyError::Shape);
        }
        Ok(Self(key.to_owned()))
    }

    /// The key itself, for the `x-api-key` header and nothing else.
    pub(super) fn expose(&self) -> &str {
        &self.0
    }

    /// `text` with any copy of the key replaced, for text that came back from the
    /// API and is about to be shown.
    pub fn scrub(&self, text: &str) -> String {
        text.replace(&self.0, "[redacted]")
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey([redacted])")
    }
}

impl fmt::Display for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl Drop for ApiKey {
    fn drop(&mut self) {
        // SAFETY: zero bytes are valid UTF-8, and the string is not used again.
        let bytes = unsafe { self.0.as_mut_vec() };
        for byte in bytes.iter_mut() {
            // Volatile, so the writes are not optimized away as dead stores.
            // SAFETY: `byte` is a valid, aligned, exclusive reference.
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123456789_-ABCDEF";

    #[test]
    fn formatting_never_shows_the_key() {
        let key = ApiKey::parse(KEY).unwrap();
        for printed in [
            format!("{key}"),
            format!("{key:?}"),
            format!("{key:#?}"),
            format!("{:?}", Some(&key)),
            format!("{:?}", std::sync::Arc::new(key)),
        ] {
            assert!(!printed.contains("sk-ant"), "{printed}");
            assert!(printed.contains("redacted"), "{printed}");
        }
    }

    #[test]
    fn checks_the_shape() {
        assert_eq!(ApiKey::parse("  ").unwrap_err(), KeyError::Empty);
        assert_eq!(
            ApiKey::parse("sk-proj-abcdefghijklmnop").unwrap_err(),
            KeyError::Prefix
        );
        assert_eq!(ApiKey::parse("sk-ant-short").unwrap_err(), KeyError::Shape);
        assert_eq!(
            ApiKey::parse("sk-ant-api03-has a space in it").unwrap_err(),
            KeyError::Shape
        );
        assert_eq!(
            ApiKey::parse(&format!("sk-ant-{}", "a".repeat(300))).unwrap_err(),
            KeyError::Shape
        );
        let key = ApiKey::parse(&format!("  {KEY}\n")).unwrap();
        assert_eq!(key.expose(), KEY);
    }

    #[test]
    fn scrubs_copies_from_text() {
        let key = ApiKey::parse(KEY).unwrap();
        assert_eq!(key.scrub(&format!("bad key {KEY}!")), "bad key [redacted]!");
    }
}
