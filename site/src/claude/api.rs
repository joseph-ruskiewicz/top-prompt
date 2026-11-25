//! The Messages API over HTTPS: the request, the streamed response and its errors.
//!
//! There is no official Rust SDK, so this follows the raw HTTP API: a `POST` to
//! `/v1/messages` with `stream: true`, read as server-sent events. Event and error
//! shapes follow the Claude API docs ("Streaming messages" and "Claude API errors").

use std::{collections::HashMap, fmt, time::Duration};

use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{Value, json};

use super::{key::ApiKey, sse, tools};

/// The API version the requests and the parsing are written against.
pub const API_VERSION: &str = "2026-10-21";

/// The cap on each response, thinking included. The visitor pays for it, so it is
/// modest; a longer answer stops with a note.
pub const MAX_TOKENS: u32 = 8192;

/// The only host the key is ever sent to in a release build.
pub const PRODUCTION_URL: &str = "https://api.anthropic.com";

/// The models a visitor can pick. Anything else from the browser is rejected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Model {
    Opus,
    #[default]
    Sonnet,
    Haiku,
}

impl Model {
    pub const ALL: [Self; 3] = [Self::Opus, Self::Sonnet, Self::Haiku];

    pub fn id(self) -> &'static str {
        match self {
            Self::Opus => "claude-opus-5-5",
            Self::Sonnet => "claude-sonnet-5-5",
            Self::Haiku => "claude-haiku-5-5",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Opus => "Claude Opus 5.5",
            Self::Sonnet => "Claude Sonnet 5.5",
            Self::Haiku => "Claude Haiku 5.5",
        }
    }

    /// The model with the id `id`, if it is on the list.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|model| model.id() == id)
    }
}

/// Where requests go. Only debug builds and tests can point it anywhere but the
/// production API, and only from the server's environment or test code, never
/// from anything a browser sends.
#[derive(Clone, Debug)]
pub struct Endpoint(String);

impl Endpoint {
    pub fn production() -> Self {
        Self(PRODUCTION_URL.to_owned())
    }

    /// The production API, or in a debug build the `DEMO_ANTHROPIC_BASE_URL`
    /// environment variable when it is set.
    pub fn from_env() -> Self {
        #[cfg(debug_assertions)]
        if let Ok(url) = std::env::var("DEMO_ANTHROPIC_BASE_URL") {
            return Self(url.trim_end_matches('/').to_owned());
        }
        Self::production()
    }

    /// A local stand-in for the API, such as the fake server.
    #[cfg(any(test, debug_assertions))]
    pub fn local(url: &str) -> Self {
        Self(url.trim_end_matches('/').to_owned())
    }

    pub fn is_production(&self) -> bool {
        self.0 == PRODUCTION_URL
    }
}

/// The HTTP client.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    endpoint: Endpoint,
}

impl Client {
    pub fn new(endpoint: Endpoint) -> Self {
        // reqwest is built without a crypto provider; this installs ring. It fails
        // only if a provider is already installed, which is just as good.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder()
            // A redirect would carry the x-api-key header to wherever it points;
            // the API does not redirect, so none are followed.
            .redirect(reqwest::redirect::Policy::none())
            // Proxy settings in the environment are ignored: the key goes straight
            // to the API.
            .no_proxy()
            .https_only(endpoint.is_production())
            .connect_timeout(Duration::from_secs(10))
            // Thinking can leave the stream quiet for a while, though the API sends
            // pings; a minute of silence means it is gone.
            .read_timeout(Duration::from_secs(60))
            .user_agent("topcoat-prompt-demo")
            .build()
            .expect("the HTTP client builds");
        Self { http, endpoint }
    }

    /// Sends a streaming request and returns the response once its status is a
    /// success, or the error the API returned.
    pub async fn stream(&self, key: &ApiKey, body: &Value) -> Result<Stream, ApiError> {
        let mut headers = HeaderMap::new();
        let mut key_header =
            HeaderValue::from_str(key.expose()).map_err(|_| ApiError::transport())?;
        // Sensitive values print as `Sensitive` in reqwest's and http's Debug output.
        key_header.set_sensitive(true);
        headers.insert("x-api-key", key_header);
        headers.insert("anthropic-version", HeaderValue::from_static(API_VERSION));
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let response = self
            .http
            .post(format!("{}/v1/messages", self.endpoint.0))
            .headers(headers)
            .body(body.to_string())
            .send()
            .await
            .map_err(|_| ApiError::transport())?;

        let status = response.status();
        if status.is_success() {
            return Ok(Stream {
                response,
                parser: sse::Parser::default(),
            });
        }
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok());
        let body = read_capped(response, 64 * 1024).await;
        Err(ApiError::from_response(
            status.as_u16(),
            &body,
            retry_after,
            key,
        ))
    }
}

/// Reads at most `limit` bytes of an error body.
async fn read_capped(mut response: reqwest::Response, limit: usize) -> String {
    let mut body = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        body.extend_from_slice(&chunk);
        if body.len() >= limit {
            body.truncate(limit);
            break;
        }
    }
    String::from_utf8_lossy(&body).into_owned()
}

/// A response being streamed.
pub struct Stream {
    response: reqwest::Response,
    parser: sse::Parser,
}

impl Stream {
    /// The events in the next chunk, or `None` at the end of the stream.
    pub async fn next(&mut self) -> Result<Option<Vec<sse::Event>>, ApiError> {
        match self.response.chunk().await {
            Ok(Some(chunk)) => self
                .parser
                .feed(&chunk)
                .map(Some)
                .map_err(|_| ApiError::protocol()),
            Ok(None) => Ok(None),
            Err(_) => Err(ApiError::transport()),
        }
    }
}

/// The request body for one step of a conversation.
pub fn request_body(model: Model, messages: &[Value]) -> Value {
    json!({
        "model": model.id(),
        "max_tokens": MAX_TOKENS,
        "stream": true,
        "system": SYSTEM_PROMPT,
        // Thinking is on by default on these models but hidden; "summarized" returns
        // a readable summary of it for the reasoning component.
        "thinking": { "type": "adaptive", "display": "summarized" },
        "tools": tools::definitions(),
        "messages": messages,
    })
}

const SYSTEM_PROMPT: &str = "You are the assistant in a live demo of topcoat-prompt, a \
library of chat interface components for Topcoat, a Rust web framework. The page shows \
your reasoning, your tool calls and your Markdown answer with the library's components. \
Answer concisely. You have two tools: search_components looks up topcoat-prompt \
components by keyword, and calculate evaluates arithmetic exactly. Use them when they \
help answer.";

/// The kinds of error the API documents, by `error.type`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// 400 `invalid_request_error`, also used for a spend limit the organization set.
    InvalidRequest,
    /// 401 `authentication_error`: a malformed, revoked or expired key.
    Authentication,
    /// 402 `billing_error`, such as no credit left.
    Billing,
    /// 403 `permission_error`.
    Permission,
    /// 404 `not_found_error`, including a model the organization cannot use.
    NotFound,
    /// 413 `request_too_large`.
    RequestTooLarge,
    /// 429 `rate_limit_error`, including the usage tier's monthly spend cap.
    RateLimit,
    /// 500 `api_error`.
    Api,
    /// 504 `timeout_error`.
    Timeout,
    /// 529 `overloaded_error`.
    Overloaded,
    /// A type the docs may add later.
    Other(String),
}

impl ErrorKind {
    fn from_type(error_type: &str) -> Self {
        match error_type {
            "invalid_request_error" => Self::InvalidRequest,
            "authentication_error" => Self::Authentication,
            "billing_error" => Self::Billing,
            "permission_error" => Self::Permission,
            "not_found_error" => Self::NotFound,
            "request_too_large" => Self::RequestTooLarge,
            "rate_limit_error" => Self::RateLimit,
            "api_error" => Self::Api,
            "timeout_error" => Self::Timeout,
            "overloaded_error" => Self::Overloaded,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The kind for a status whose body had no error type.
    fn from_status(status: u16) -> Self {
        match status {
            400 => Self::InvalidRequest,
            401 => Self::Authentication,
            402 => Self::Billing,
            403 => Self::Permission,
            404 => Self::NotFound,
            413 => Self::RequestTooLarge,
            429 => Self::RateLimit,
            504 => Self::Timeout,
            529 => Self::Overloaded,
            _ => Self::Api,
        }
    }
}

/// A failed request or stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiError {
    /// An error the API returned, as a response or as an `error` event mid-stream.
    Api {
        /// The HTTP status, or `None` for an error event in a stream.
        status: Option<u16>,
        kind: ErrorKind,
        /// The API's message, with any copy of the key removed.
        message: String,
        /// Seconds from the `retry-after` header.
        retry_after: Option<u64>,
    },
    /// The API could not be reached, or the connection dropped.
    Transport,
    /// The stream did not follow the documented format.
    Protocol,
}

impl ApiError {
    fn transport() -> Self {
        Self::Transport
    }

    fn protocol() -> Self {
        Self::Protocol
    }

    /// The error for a response with a non-success `status` and `body`.
    pub fn from_response(status: u16, body: &str, retry_after: Option<u64>, key: &ApiKey) -> Self {
        let parsed = serde_json::from_str::<Value>(body).ok();
        let error = parsed.as_ref().and_then(|body| body.get("error"));
        let kind = error
            .and_then(|error| error.get("type"))
            .and_then(Value::as_str)
            .map_or_else(|| ErrorKind::from_status(status), ErrorKind::from_type);
        let message = error
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        Self::Api {
            status: Some(status),
            kind,
            message: key.scrub(&truncate(message, 500)),
            retry_after,
        }
    }

    /// The error for an `error` event in a stream.
    pub fn from_event(data: &Value, key: &ApiKey) -> Self {
        let error = data.get("error");
        let kind = error
            .and_then(|error| error.get("type"))
            .and_then(Value::as_str)
            .map_or(ErrorKind::Api, ErrorKind::from_type);
        let message = error
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        Self::Api {
            status: None,
            kind,
            message: key.scrub(&truncate(message, 500)),
            retry_after: None,
        }
    }

    /// A sentence for the visitor about what went wrong and what to do.
    pub fn explain(&self, model: Model) -> String {
        let Self::Api {
            kind,
            message,
            retry_after,
            status,
        } = self
        else {
            return match self {
                Self::Transport => {
                    "Could not reach the Claude API, or the connection dropped. Try again."
                        .to_owned()
                }
                _ => "The response stream was malformed. Try again.".to_owned(),
            };
        };
        let mut text = match kind {
            ErrorKind::Authentication => {
                "The API key was rejected: it may be mistyped, revoked or expired. Clear it \
                 and enter another."
                    .to_owned()
            }
            ErrorKind::Billing => "Your Anthropic account has a billing problem, such as no \
                 credit left. Check billing in the Claude Console."
                .to_owned(),
            ErrorKind::Permission => format!(
                "This key is not allowed to use {}. Check the key's workspace in the Claude \
                 Console.",
                model.label()
            ),
            ErrorKind::NotFound => format!(
                "{} is not available to your organization. Pick another model.",
                model.label()
            ),
            ErrorKind::RateLimit => match retry_after {
                Some(seconds) => {
                    format!("Your account hit a rate limit. Try again in {seconds} seconds.")
                }
                // A spend-cap 429 has no retry-after and keeps failing.
                None => "Your account hit a rate limit or its monthly spend cap.".to_owned(),
            },
            ErrorKind::Overloaded => "The Claude API is overloaded right now. Try again in a \
                 moment, or pick another model."
                .to_owned(),
            ErrorKind::Api => "The Claude API had an internal error. Try again.".to_owned(),
            ErrorKind::Timeout => "The request timed out. Try again.".to_owned(),
            ErrorKind::RequestTooLarge => {
                "The conversation is too large to send. Clear it and start again.".to_owned()
            }
            ErrorKind::InvalidRequest => "The API rejected the request.".to_owned(),
            ErrorKind::Other(kind) => format!("The API returned an error ({kind})."),
        };
        let code = match (status, kind) {
            (Some(status), _) => format!(" ({status})"),
            (None, _) => " (mid-stream)".to_owned(),
        };
        text.push_str(&code);
        if !message.is_empty() {
            text.push_str(": ");
            text.push_str(message);
        }
        text
    }

    /// Whether the key itself is the problem, so the visitor should replace it.
    pub fn is_key_problem(&self) -> bool {
        matches!(
            self,
            Self::Api {
                kind: ErrorKind::Authentication,
                ..
            }
        )
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.explain(Model::default()))
    }
}

fn truncate(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_owned(),
    }
}

/// What a stream event changed, for the page to show.
#[derive(Debug, PartialEq)]
pub enum Update {
    ThinkingStarted {
        index: usize,
    },
    Thinking {
        index: usize,
        text: String,
    },
    TextStarted {
        index: usize,
    },
    Text {
        index: usize,
        text: String,
    },
    ToolStarted {
        index: usize,
        id: String,
        name: String,
    },
    ToolInput {
        index: usize,
        partial: String,
    },
    BlockStopped {
        index: usize,
    },
}

/// One assistant message, built from the events of a stream.
#[derive(Default)]
pub struct Accumulator {
    /// The content blocks as the API sent them, so they go back unchanged; thinking
    /// blocks in particular must be returned exactly.
    pub content: Vec<Value>,
    /// The raw input of each tool-use block, by index, until the block stops.
    inputs: HashMap<usize, String>,
    /// Tool-use inputs that were not valid JSON, by index: the raw text.
    pub invalid_inputs: HashMap<usize, String>,
    pub stop_reason: Option<String>,
    /// `message_stop` arrived.
    pub finished: bool,
}

impl Accumulator {
    /// Applies one event. An `error` event becomes an [`ApiError`].
    pub fn apply(&mut self, event: &sse::Event, key: &ApiKey) -> Result<Option<Update>, ApiError> {
        let data: Value = serde_json::from_str(&event.data).map_err(|_| ApiError::protocol())?;
        let kind = data
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or(&event.name);
        let index = data
            .get("index")
            .and_then(Value::as_u64)
            .and_then(|index| usize::try_from(index).ok());
        Ok(match kind {
            "content_block_start" => {
                let index = index.ok_or_else(ApiError::protocol)?;
                let block = data
                    .get("content_block")
                    .cloned()
                    .ok_or_else(ApiError::protocol)?;
                // Indexes count up from 0; a gap would mean a lost event.
                if index != self.content.len() {
                    return Err(ApiError::protocol());
                }
                let update = match block.get("type").and_then(Value::as_str) {
                    Some("thinking") => Some(Update::ThinkingStarted { index }),
                    Some("text") => Some(Update::TextStarted { index }),
                    Some("tool_use") => {
                        self.inputs.insert(index, String::new());
                        Some(Update::ToolStarted {
                            index,
                            id: string_field(&block, "id"),
                            name: string_field(&block, "name"),
                        })
                    }
                    // Redacted thinking arrives whole; other blocks are kept as sent.
                    _ => None,
                };
                self.content.push(block);
                update
            }
            "content_block_delta" => {
                let index = index.ok_or_else(ApiError::protocol)?;
                let delta = data.get("delta").ok_or_else(ApiError::protocol)?;
                let block = self.content.get_mut(index).ok_or_else(ApiError::protocol)?;
                match delta.get("type").and_then(Value::as_str) {
                    Some("text_delta") => {
                        let text = string_field(delta, "text");
                        append(block, "text", &text);
                        Some(Update::Text { index, text })
                    }
                    Some("thinking_delta") => {
                        let text = string_field(delta, "thinking");
                        append(block, "thinking", &text);
                        Some(Update::Thinking { index, text })
                    }
                    Some("signature_delta") => {
                        append(block, "signature", &string_field(delta, "signature"));
                        None
                    }
                    Some("input_json_delta") => {
                        let partial = string_field(delta, "partial_json");
                        if let Some(input) = self.inputs.get_mut(&index) {
                            input.push_str(&partial);
                        }
                        Some(Update::ToolInput { index, partial })
                    }
                    // Citations and future delta types are not shown here.
                    _ => None,
                }
            }
            "content_block_stop" => {
                let index = index.ok_or_else(ApiError::protocol)?;
                if let Some(raw) = self.inputs.remove(&index) {
                    // The final input is always an object; an empty one streams no
                    // fragments at all.
                    let raw_or_empty = if raw.trim().is_empty() { "{}" } else { &raw };
                    let input = match serde_json::from_str::<Value>(raw_or_empty) {
                        Ok(input @ Value::Object(_)) => input,
                        _ => {
                            self.invalid_inputs.insert(index, raw);
                            json!({})
                        }
                    };
                    let block = self.content.get_mut(index).ok_or_else(ApiError::protocol)?;
                    block["input"] = input;
                }
                Some(Update::BlockStopped { index })
            }
            "message_delta" => {
                if let Some(reason) = data
                    .get("delta")
                    .and_then(|delta| delta.get("stop_reason"))
                    .and_then(Value::as_str)
                {
                    self.stop_reason = Some(reason.to_owned());
                }
                None
            }
            "message_stop" => {
                self.finished = true;
                None
            }
            "error" => return Err(ApiError::from_event(&data, key)),
            // message_start, ping, and event types added later.
            _ => None,
        })
    }

    /// The message to send back as the assistant's turn. Empty text blocks are left
    /// out, since the API rejects them; every other block goes back as it came.
    pub fn assistant_message(&self) -> Value {
        let content: Vec<&Value> = self
            .content
            .iter()
            .filter(|block| {
                !(block.get("type").and_then(Value::as_str) == Some("text")
                    && block
                        .get("text")
                        .and_then(Value::as_str)
                        .is_none_or(str::is_empty))
            })
            .collect();
        json!({ "role": "assistant", "content": content })
    }
}

fn string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Appends `text` to the string `field` of `block`.
fn append(block: &mut Value, field: &str, text: &str) {
    let mut current = block
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    current.push_str(text);
    block[field] = Value::String(current);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> ApiKey {
        ApiKey::parse("sk-ant-api03-test-key-0123456789abcdef").unwrap()
    }

    /// Feeds a stream through the parser and the accumulator.
    fn run(stream: &str) -> (Accumulator, Vec<Update>, Option<ApiError>) {
        let mut parser = sse::Parser::default();
        let mut message = Accumulator::default();
        let mut updates = Vec::new();
        for event in parser.feed(stream.as_bytes()).unwrap() {
            match message.apply(&event, &key()) {
                Ok(Some(update)) => updates.push(update),
                Ok(None) => {}
                Err(error) => return (message, updates, Some(error)),
            }
        }
        (message, updates, None)
    }

    // The streams below are the examples in the "Streaming messages" docs.

    #[test]
    fn accumulates_thinking_and_text() {
        let (message, updates, error) = run("event: message_start\n\
             data: {\"type\": \"message_start\", \"message\": {\"id\": \"msg_01\", \"type\": \"message\", \"role\": \"assistant\", \"content\": [], \"model\": \"claude-opus-5-5\", \"stop_reason\": null, \"stop_sequence\": null}}\n\n\
             event: content_block_start\n\
             data: {\"type\": \"content_block_start\", \"index\": 0, \"content_block\": {\"type\": \"thinking\", \"thinking\": \"\", \"signature\": \"\"}}\n\n\
             event: content_block_delta\n\
             data: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"thinking_delta\", \"thinking\": \"1071 = 2 × 462 + 147\"}}\n\n\
             event: content_block_delta\n\
             data: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"thinking_delta\", \"thinking\": \"\\n462 = 3 × 147 + 21\"}}\n\n\
             event: content_block_delta\n\
             data: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"signature_delta\", \"signature\": \"EqQBCgIYAhIM1gbcDa9GJwZA2b3hGgxBdjrkzLoky3dl1pkiMOYds\"}}\n\n\
             event: content_block_stop\n\
             data: {\"type\": \"content_block_stop\", \"index\": 0}\n\n\
             event: content_block_start\n\
             data: {\"type\": \"content_block_start\", \"index\": 1, \"content_block\": {\"type\": \"text\", \"text\": \"\"}}\n\n\
             event: ping\n\
             data: {\"type\": \"ping\"}\n\n\
             event: content_block_delta\n\
             data: {\"type\": \"content_block_delta\", \"index\": 1, \"delta\": {\"type\": \"text_delta\", \"text\": \"The GCD is **21**.\"}}\n\n\
             event: content_block_stop\n\
             data: {\"type\": \"content_block_stop\", \"index\": 1}\n\n\
             event: message_delta\n\
             data: {\"type\": \"message_delta\", \"delta\": {\"stop_reason\": \"end_turn\", \"stop_sequence\": null}}\n\n\
             event: message_stop\n\
             data: {\"type\": \"message_stop\"}\n\n");
        assert_eq!(error, None);
        assert!(message.finished);
        assert_eq!(message.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(
            message.assistant_message(),
            json!({"role": "assistant", "content": [
                {
                    "type": "thinking",
                    "thinking": "1071 = 2 × 462 + 147\n462 = 3 × 147 + 21",
                    "signature": "EqQBCgIYAhIM1gbcDa9GJwZA2b3hGgxBdjrkzLoky3dl1pkiMOYds",
                },
                { "type": "text", "text": "The GCD is **21**." },
            ]})
        );
        assert_eq!(updates[0], Update::ThinkingStarted { index: 0 });
        assert!(updates.contains(&Update::Text {
            index: 1,
            text: "The GCD is **21**.".to_owned()
        }));
    }

    #[test]
    fn accumulates_a_tool_call() {
        let (message, updates, error) = run("event: content_block_start\n\
             data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
             event: content_block_delta\n\
             data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Okay\"}}\n\n\
             event: content_block_stop\n\
             data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
             event: content_block_start\n\
             data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_01T1x1fJ34qAmk2tNTrN7Up6\",\"name\":\"get_weather\",\"input\":{}}}\n\n\
             event: content_block_delta\n\
             data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\"}}\n\n\
             event: content_block_delta\n\
             data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"location\\\":\"}}\n\n\
             event: content_block_delta\n\
             data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\" \\\"San Francisco, CA\\\"}\"}}\n\n\
             event: content_block_stop\n\
             data: {\"type\":\"content_block_stop\",\"index\":1}\n\n\
             event: message_delta\n\
             data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":89}}\n\n\
             event: message_stop\n\
             data: {\"type\":\"message_stop\"}\n\n");
        assert_eq!(error, None);
        assert_eq!(message.stop_reason.as_deref(), Some("tool_use"));
        assert_eq!(
            message.content[1],
            json!({
                "type": "tool_use",
                "id": "toolu_01T1x1fJ34qAmk2tNTrN7Up6",
                "name": "get_weather",
                "input": { "location": "San Francisco, CA" },
            })
        );
        assert!(message.invalid_inputs.is_empty());
        assert!(updates.contains(&Update::ToolStarted {
            index: 1,
            id: "toolu_01T1x1fJ34qAmk2tNTrN7Up6".to_owned(),
            name: "get_weather".to_owned(),
        }));
    }

    #[test]
    fn keeps_invalid_tool_input_aside() {
        let (message, _, error) = run(
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"t\",\"name\":\"calculate\",\"input\":{}}}\n\n\
             data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"expression\\\": \\\"1+\"}}\n\n\
             data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        );
        assert_eq!(error, None);
        assert_eq!(message.content[0]["input"], json!({}));
        assert_eq!(message.invalid_inputs[&0], "{\"expression\": \"1+");
    }

    #[test]
    fn an_error_event_ends_the_stream() {
        let (message, _, error) = run("event: content_block_start\n\
             data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
             event: content_block_delta\n\
             data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hel\"}}\n\n\
             event: error\n\
             data: {\"type\": \"error\", \"error\": {\"type\": \"overloaded_error\", \"message\": \"Overloaded\"}}\n\n");
        assert_eq!(message.content[0]["text"], "Hel");
        assert_eq!(
            error,
            Some(ApiError::Api {
                status: None,
                kind: ErrorKind::Overloaded,
                message: "Overloaded".to_owned(),
                retry_after: None,
            })
        );
    }

    #[test]
    fn unknown_events_and_deltas_are_ignored() {
        let (message, updates, error) = run(
            "event: something_new\ndata: {\"type\":\"something_new\"}\n\n\
             data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"redacted_thinking\",\"data\":\"abc\"}}\n\n\
             data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"new_delta\"}}\n\n\
             data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        );
        assert_eq!(error, None);
        assert_eq!(
            message.content,
            [json!({"type": "redacted_thinking", "data": "abc"})]
        );
        assert_eq!(updates, [Update::BlockStopped { index: 0 }]);
    }

    #[test]
    fn maps_the_documented_error_responses() {
        let key = key();
        let cases = [
            (400, "invalid_request_error", ErrorKind::InvalidRequest),
            (401, "authentication_error", ErrorKind::Authentication),
            (402, "billing_error", ErrorKind::Billing),
            (403, "permission_error", ErrorKind::Permission),
            (404, "not_found_error", ErrorKind::NotFound),
            (413, "request_too_large", ErrorKind::RequestTooLarge),
            (429, "rate_limit_error", ErrorKind::RateLimit),
            (500, "api_error", ErrorKind::Api),
            (504, "timeout_error", ErrorKind::Timeout),
            (529, "overloaded_error", ErrorKind::Overloaded),
        ];
        for (status, error_type, kind) in cases {
            let body = format!(
                "{{\"type\":\"error\",\"error\":{{\"type\":\"{error_type}\",\"message\":\"m\"}},\"request_id\":\"req_1\"}}"
            );
            let error = ApiError::from_response(status, &body, None, &key);
            assert_eq!(
                error,
                ApiError::Api {
                    status: Some(status),
                    kind: kind.clone(),
                    message: "m".to_owned(),
                    retry_after: None
                }
            );
            // A body that is not JSON (such as a proxy's page) falls back to the status.
            let ApiError::Api { kind: fallback, .. } =
                ApiError::from_response(status, "<html>", None, &key)
            else {
                panic!("an API error");
            };
            if status != 500 {
                assert_eq!(fallback, kind, "{status}");
            }
        }
        let error = ApiError::from_response(429, "{}", Some(30), &key);
        assert!(error.explain(Model::Sonnet).contains("30 seconds"));
        assert!(error.explain(Model::Sonnet).contains("(429)"));
    }

    #[test]
    fn error_messages_never_carry_the_key() {
        let key = key();
        let body = format!(
            "{{\"type\":\"error\",\"error\":{{\"type\":\"authentication_error\",\"message\":\"invalid x-api-key {}\"}}}}",
            key.expose()
        );
        let error = ApiError::from_response(401, &body, None, &key);
        for printed in [
            error.explain(Model::Opus),
            format!("{error}"),
            format!("{error:?}"),
        ] {
            assert!(!printed.contains(key.expose()), "{printed}");
        }
        assert!(error.is_key_problem());
    }

    #[test]
    fn request_body_follows_the_docs() {
        let body = request_body(
            Model::Haiku,
            &[json!({"role": "user", "content": [{"type": "text", "text": "hi"}]})],
        );
        assert_eq!(body["model"], "claude-haiku-5-5");
        assert_eq!(body["max_tokens"], MAX_TOKENS);
        assert_eq!(body["stream"], true);
        assert_eq!(
            body["thinking"],
            json!({"type": "adaptive", "display": "summarized"})
        );
        assert!(body.get("tool_choice").is_none());
        for tool in body["tools"].as_array().unwrap() {
            assert_eq!(tool["eager_input_streaming"], true);
        }
    }

    #[test]
    fn only_listed_models_are_accepted() {
        assert_eq!(Model::from_id("claude-sonnet-5-5"), Some(Model::Sonnet));
        assert_eq!(Model::from_id("claude-opus-5-5"), Some(Model::Opus));
        assert_eq!(Model::from_id("claude-haiku-5-5"), Some(Model::Haiku));
        assert_eq!(Model::from_id("claude-fable-5-2"), None);
        assert_eq!(Model::from_id(""), None);
        assert_eq!(Model::default(), Model::Sonnet);
    }

    #[test]
    fn the_production_endpoint_is_https() {
        assert!(PRODUCTION_URL.starts_with("https://api.anthropic.com"));
        assert!(Endpoint::production().is_production());
        assert!(!Endpoint::local("http://127.0.0.1:1").is_production());
    }
}
