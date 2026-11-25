//! The visitor's API key, kept out of every printed form.

use std::fmt;

use super::provider::Provider;

/// The prefix of every Anthropic API key.
const ANTHROPIC_PREFIX: &str = "sk-ant-";

/// The prefix of every OpenAI API key (project, service account or legacy user).
const OPENAI_PREFIX: &str = "sk-";

/// The longest key accepted, in bytes. Real keys are a few hundred at most.
const MAX_LEN: usize = 256;

/// The shortest run of a key's characters that [`ApiKey::scrub`] removes from text.
/// Shorter than any key, and long enough that ordinary words rarely match.
const SCRUB_RUN: usize = 8;

/// An API key a visitor entered, for one provider.
///
/// `Debug` and `Display` print `[redacted]`, so the key cannot reach a log line, an
/// error message or a panic through formatting. It is not `Clone`, `Serialize` or
/// `Deserialize`: share it behind an `Arc`, and the only way to read it is
/// [`expose`](Self::expose), which the HTTP client calls to set the auth header.
/// The bytes are overwritten when the key is dropped.
pub struct ApiKey(String);

/// Why a key was rejected before any request was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyError {
    Empty,
    /// It does not start with the selected provider's prefix.
    Prefix(Provider),
    /// It is the other provider's key.
    WrongProvider(Provider),
    /// It is too long, or has spaces or other characters keys do not use.
    Shape(Provider),
}

impl KeyError {
    /// The message shown under the key field.
    pub fn message(self) -> &'static str {
        match self {
            Self::Empty => "Paste a key first.",
            Self::Prefix(Provider::Anthropic) => "Anthropic API keys start with sk-ant-.",
            Self::Prefix(Provider::OpenAi) => "OpenAI API keys start with sk-.",
            Self::WrongProvider(Provider::Anthropic) => {
                "That looks like an Anthropic key. Switch the provider above, or paste an OpenAI key."
            }
            Self::WrongProvider(Provider::OpenAi) => {
                "That looks like an OpenAI key. Switch the provider above, or paste an Anthropic key."
            }
            Self::Shape(Provider::Anthropic) => "That does not look like an Anthropic API key.",
            Self::Shape(Provider::OpenAi) => "That does not look like an OpenAI API key.",
        }
    }
}

/// The provider whose prefix `key` carries, if any.
fn provider_of(key: &str) -> Option<Provider> {
    if key.starts_with(ANTHROPIC_PREFIX) {
        Some(Provider::Anthropic)
    } else if key.starts_with(OPENAI_PREFIX) {
        Some(Provider::OpenAi)
    } else {
        None
    }
}

/// The prefix every key for `provider` starts with.
pub fn prefix(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => ANTHROPIC_PREFIX,
        Provider::OpenAi => OPENAI_PREFIX,
    }
}

impl ApiKey {
    /// Checks the shape of a key pasted for `provider`. Surrounding whitespace is
    /// ignored. This does not check that the key works; the first request does.
    pub fn parse(provider: Provider, raw: &str) -> Result<Self, KeyError> {
        let key = raw.trim();
        if key.is_empty() {
            return Err(KeyError::Empty);
        }
        match provider_of(key) {
            None => return Err(KeyError::Prefix(provider)),
            Some(other) if other != provider => return Err(KeyError::WrongProvider(other)),
            Some(_) => {}
        }
        let shaped = key.len() <= MAX_LEN
            && key.len() > prefix(provider).len() + 8
            && key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
        if !shaped {
            return Err(KeyError::Shape(provider));
        }
        Ok(Self(key.to_owned()))
    }

    /// The key itself, for the auth header and nothing else.
    pub(super) fn expose(&self) -> &str {
        &self.0
    }

    /// `text` with any copy of the key, or of a run of [`SCRUB_RUN`] or more of its
    /// characters, replaced, for text that came back from the API and is about to be
    /// shown. An API may echo a key masked or cut short, so partial copies go too.
    pub fn scrub(&self, text: &str) -> String {
        let text_bytes = text.as_bytes();
        let key = self.0.as_bytes();
        // `run[j]` is the length of the common substring of the text and the key
        // ending at the current text byte and at key byte `j`.
        let mut run = vec![0usize; key.len()];
        let mut hidden = vec![false; text_bytes.len()];
        for (i, &byte) in text_bytes.iter().enumerate() {
            for j in (0..key.len()).rev() {
                run[j] = if key[j] != byte {
                    0
                } else if j == 0 {
                    1
                } else {
                    run[j - 1] + 1
                };
                if run[j] >= SCRUB_RUN {
                    hidden[i + 1 - run[j]..=i].fill(true);
                }
            }
        }
        // Keys are ASCII, so every hidden run starts and ends on a character
        // boundary.
        let mut scrubbed = String::with_capacity(text.len());
        let mut start = 0;
        while start < text_bytes.len() {
            let end = hidden[start..]
                .iter()
                .position(|&flag| flag != hidden[start])
                .map_or(text_bytes.len(), |length| start + length);
            if hidden[start] {
                scrubbed.push_str("[redacted]");
            } else {
                scrubbed.push_str(&text[start..end]);
            }
            start = end;
        }
        scrubbed
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
        let key = ApiKey::parse(Provider::Anthropic, KEY).unwrap();
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
        assert_eq!(
            ApiKey::parse(Provider::Anthropic, "  ").unwrap_err(),
            KeyError::Empty
        );
        assert_eq!(
            ApiKey::parse(Provider::Anthropic, "key-abcdefghijklmnop").unwrap_err(),
            KeyError::Prefix(Provider::Anthropic)
        );
        assert_eq!(
            ApiKey::parse(Provider::Anthropic, "sk-ant-short").unwrap_err(),
            KeyError::Shape(Provider::Anthropic)
        );
        assert_eq!(
            ApiKey::parse(Provider::Anthropic, "sk-ant-api03-has a space in it").unwrap_err(),
            KeyError::Shape(Provider::Anthropic)
        );
        assert_eq!(
            ApiKey::parse(Provider::Anthropic, &format!("sk-ant-{}", "a".repeat(300))).unwrap_err(),
            KeyError::Shape(Provider::Anthropic)
        );
        let key = ApiKey::parse(Provider::Anthropic, &format!("  {KEY}\n")).unwrap();
        assert_eq!(key.expose(), KEY);
    }

    #[test]
    fn openai_keys_parse() {
        let key = ApiKey::parse(Provider::OpenAi, "sk-proj-abcdefghijklmnopqrstuvwx").unwrap();
        assert_eq!(key.expose(), "sk-proj-abcdefghijklmnopqrstuvwx");
        assert_eq!(
            ApiKey::parse(Provider::OpenAi, "pk-proj-abcdefghijklmnopqrstuvwx").unwrap_err(),
            KeyError::Prefix(Provider::OpenAi)
        );
        assert_eq!(
            ApiKey::parse(Provider::OpenAi, "sk-short").unwrap_err(),
            KeyError::Shape(Provider::OpenAi)
        );
    }

    #[test]
    fn swapped_keys_name_their_provider() {
        assert_eq!(
            ApiKey::parse(Provider::OpenAi, KEY).unwrap_err(),
            KeyError::WrongProvider(Provider::Anthropic)
        );
        assert_eq!(
            ApiKey::parse(Provider::Anthropic, "sk-proj-abcdefghijklmnopqrstuvwx").unwrap_err(),
            KeyError::WrongProvider(Provider::OpenAi)
        );
        assert_eq!(
            KeyError::WrongProvider(Provider::Anthropic).message(),
            "That looks like an Anthropic key. Switch the provider above, or paste an OpenAI key."
        );
        assert_eq!(
            KeyError::WrongProvider(Provider::OpenAi).message(),
            "That looks like an OpenAI key. Switch the provider above, or paste an Anthropic key."
        );
        assert!(
            KeyError::Prefix(Provider::Anthropic)
                .message()
                .contains("sk-ant-")
        );
    }

    #[test]
    fn scrubs_copies_from_text() {
        let key = ApiKey::parse(Provider::Anthropic, KEY).unwrap();
        assert_eq!(key.scrub(&format!("bad key {KEY}!")), "bad key [redacted]!");
        assert_eq!(key.scrub("no key here"), "no key here");
        // Partial copies: a tail, a masked form, and a copy cut short.
        let tail = &KEY[KEY.len() - 20..];
        assert_eq!(
            key.scrub(&format!("ends in {tail}.")),
            "ends in [redacted]."
        );
        assert_eq!(
            key.scrub(&format!("{}****{}", &KEY[..12], &KEY[KEY.len() - 4..])),
            format!("[redacted]****{}", &KEY[KEY.len() - 4..])
        );
        assert_eq!(key.scrub(&KEY[..30]), "[redacted]");
        // Text around the key keeps its characters, multi-byte ones included.
        assert_eq!(key.scrub(&format!("é{KEY}ü")), "é[redacted]ü");
    }
}
