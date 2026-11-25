//! `/full/live`: the chat demo answered by the real Claude API, on the visitor's own
//! API key.
//!
//! Where the key goes: the browser sends it once, in the body of a `POST` to the
//! `save_key` procedure. The server keeps it in memory, in the [`chat::Visitor`] for
//! the visitor's session cookie, and sends it only in the `x-api-key` header of
//! requests to `https://api.anthropic.com`. It is never written to disk, a cookie, a
//! signal, a URL or a log, and its type redacts it from `Debug` and `Display`.

pub mod anthropic;
pub mod api;
pub mod chat;
pub mod cookie;
#[cfg(any(test, debug_assertions))]
pub mod fake;
pub mod key;
pub mod provider;
pub mod sse;
pub mod tools;
pub mod view;

#[cfg(test)]
mod tests;
