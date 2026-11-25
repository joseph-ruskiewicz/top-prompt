//! What every provider's API shares: the HTTP client, the streamed response, the
//! errors and the updates a stream produces for the page. The wire formats are in
//! the adapter modules, [`super::anthropic`] and [`super::openai`].

use std::time::Duration;

use reqwest::header::HeaderValue;
use serde_json::Value;

use super::{
    anthropic,
    key::ApiKey,
    openai,
    provider::{Model, Provider},
    sse,
};

/// The cap on each response, thinking included. The visitor pays for it, so it is
/// modest; a longer answer stops with a note.
pub const MAX_TOKENS: u32 = 8192;

/// The instructions sent with every request, whichever the provider.
pub const SYSTEM_PROMPT: &str = "You are the assistant in a live demo of topcoat-prompt, a \
library of chat interface components for Topcoat, a Rust web framework. The page shows \
your reasoning, your tool calls and your Markdown answer with the library's components. \
Answer concisely. You have two tools: search_components looks up topcoat-prompt \
components by keyword, and calculate evaluates arithmetic exactly. Use them when they \
help answer.";

/// The production base URL of `provider`'s API.
fn production_url(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => anthropic::PRODUCTION_URL,
        Provider::OpenAi => openai::PRODUCTION_URL,
    }
}

/// Where one provider's requests go. Only debug builds and tests can point it
/// anywhere but the production API, and only from the server's environment or test
/// code, never from anything a browser sends.
#[derive(Clone, Debug)]
pub struct Endpoint(String);

impl Endpoint {
    pub fn production(provider: Provider) -> Self {
        Self(production_url(provider).to_owned())
    }

    /// The production API, or in a debug build the `DEMO_ANTHROPIC_BASE_URL` or
    /// `DEMO_OPENAI_BASE_URL` environment variable when it is set.
    pub fn from_env(provider: Provider) -> Self {
        #[cfg(debug_assertions)]
        {
            let variable = match provider {
                Provider::Anthropic => "DEMO_ANTHROPIC_BASE_URL",
                Provider::OpenAi => "DEMO_OPENAI_BASE_URL",
            };
            if let Ok(url) = std::env::var(variable) {
                return Self(url.trim_end_matches('/').to_owned());
            }
        }
        Self::production(provider)
    }

    /// A local stand-in for the API, such as the fake server.
    #[cfg(any(test, debug_assertions))]
    pub fn local(url: &str) -> Self {
        Self(url.trim_end_matches('/').to_owned())
    }

    /// Whether this is `provider`'s production API.
    pub fn is_production(&self, provider: Provider) -> bool {
        self.0 == production_url(provider)
    }
}

/// Where each provider's requests go.
#[derive(Clone, Debug)]
pub struct Endpoints {
    pub anthropic: Endpoint,
    pub openai: Endpoint,
}

/// The HTTP client.
#[derive(Clone)]
pub struct Client {
    anthropic: Route,
    openai: Route,
}

/// The client for one provider's endpoint.
#[derive(Clone)]
struct Route {
    http: reqwest::Client,
    endpoint: Endpoint,
}

impl Route {
    fn new(provider: Provider, endpoint: Endpoint) -> Self {
        let http = reqwest::Client::builder()
            // A redirect would carry the key's header to wherever it points; the
            // APIs do not redirect, so none are followed.
            .redirect(reqwest::redirect::Policy::none())
            // Proxy settings in the environment are ignored: the key goes straight
            // to the API.
            .no_proxy()
            .https_only(endpoint.is_production(provider))
            .connect_timeout(Duration::from_secs(10))
            // Thinking can leave the stream quiet for a while, though the API sends
            // pings; a minute of silence means it is gone.
            .read_timeout(Duration::from_secs(60))
            .user_agent("topcoat-prompt-demo")
            .build()
            .expect("the HTTP client builds");
        Self { http, endpoint }
    }
}

impl Client {
    pub fn new(endpoints: Endpoints) -> Self {
        // reqwest is built without a crypto provider; this installs ring. It fails
        // only if a provider is already installed, which is just as good.
        let _ = rustls::crypto::ring::default_provider().install_default();
        Self {
            anthropic: Route::new(Provider::Anthropic, endpoints.anthropic),
            openai: Route::new(Provider::OpenAi, endpoints.openai),
        }
    }

    /// Sends a streaming request to `provider`'s API and returns the response once
    /// its status is a success, or the error the API returned.
    pub async fn stream(
        &self,
        provider: Provider,
        key: &ApiKey,
        body: &Value,
    ) -> Result<Stream, ApiError> {
        let route = match provider {
            Provider::Anthropic => &self.anthropic,
            Provider::OpenAi => &self.openai,
        };
        let (path, mut headers) = match provider {
            Provider::Anthropic => (anthropic::PATH, anthropic::headers(key)?),
            Provider::OpenAi => (openai::PATH, openai::headers(key)?),
        };
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let response = route
            .http
            .post(format!("{}{path}", route.endpoint.0))
            .headers(headers)
            .body(body.to_string())
            .send()
            .await
            .map_err(|_| ApiError::Transport)?;

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
        Err(provider.parse_error(Some(status.as_u16()), &body, retry_after, key))
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
                .map_err(|_| ApiError::Protocol),
            Ok(None) => Ok(None),
            Err(_) => Err(ApiError::Transport),
        }
    }
}

/// The kinds of error the APIs report. Each adapter maps its provider's statuses and
/// codes onto them: [`super::anthropic`] from Anthropic's `error.type`s and
/// [`super::openai`] from OpenAI's statuses and `error.code`s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The request was rejected as invalid (400). Anthropic also uses it for a spend
    /// limit the organization set.
    InvalidRequest,
    /// The key is malformed, revoked or expired (401).
    Authentication,
    /// The account cannot pay, such as no credit left: Anthropic's 402
    /// `billing_error`, or OpenAI's 429 `insufficient_quota`.
    Billing,
    /// The key may not do this (403).
    Permission,
    /// The model or resource is not available to the account (404, or OpenAI's
    /// `model_not_found`).
    NotFound,
    /// The request is too large (413).
    RequestTooLarge,
    /// Too many requests (429). For Anthropic this includes the usage tier's
    /// monthly spend cap.
    RateLimit,
    /// The provider had an internal error (500).
    Api,
    /// The request timed out (504).
    Timeout,
    /// The provider is overloaded: Anthropic's 529, OpenAI's 503.
    Overloaded,
    /// A type or code the docs may add later.
    Other(String),
}

/// A failed request or stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiError {
    /// An error the API returned, as a response or as an error event mid-stream.
    /// Adapters build it with [`ApiError::api`], which removes the key from the
    /// message; never construct it directly from API text.
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
    /// An error the API returned. The message is shortened and any copy of the key
    /// is removed from it, so every adapter's errors are safe to show.
    pub fn api(
        status: Option<u16>,
        kind: ErrorKind,
        message: &str,
        retry_after: Option<u64>,
        key: &ApiKey,
    ) -> Self {
        Self::Api {
            status,
            kind,
            // Scrubbed before it is shortened, so a cut cannot leave half a key.
            message: truncate(&key.scrub(message), 500),
            retry_after,
        }
    }

    /// A sentence for the visitor about what went wrong and what to do.
    pub fn explain(&self, model: Model) -> String {
        let provider = model.provider().label();
        let Self::Api {
            kind,
            message,
            retry_after,
            status,
        } = self
        else {
            return match self {
                Self::Transport => format!(
                    "Could not reach the {provider} API, or the connection dropped. Try again."
                ),
                _ => "The response stream was malformed. Try again.".to_owned(),
            };
        };
        let mut text = match kind {
            ErrorKind::Authentication => {
                "The API key was rejected: it may be mistyped, revoked or expired. Clear it \
                 and enter another."
                    .to_owned()
            }
            ErrorKind::Billing => format!(
                "Your {provider} account has a billing problem, such as no credit left. {}",
                model.provider().billing_hint()
            ),
            ErrorKind::Permission => format!(
                "This key is not allowed to use {}. {}",
                model.label(),
                model.provider().permission_hint()
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
            ErrorKind::Overloaded => format!(
                "The {provider} API is overloaded right now. Try again in a moment, or pick \
                 another model."
            ),
            ErrorKind::Api => format!("The {provider} API had an internal error. Try again."),
            ErrorKind::Timeout => "The request timed out. Try again.".to_owned(),
            ErrorKind::RequestTooLarge => {
                "The conversation is too large to send. Clear it and start again.".to_owned()
            }
            // Most likely something in the conversation the API will not take, which
            // every later request would repeat.
            ErrorKind::InvalidRequest => {
                "The API rejected the request. Clear the conversation and try again.".to_owned()
            }
            ErrorKind::Other(kind) => format!("The API returned an error ({kind})."),
        };
        let code = match status {
            Some(status) => format!(" ({status})"),
            None => " (mid-stream)".to_owned(),
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

fn truncate(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_owned(),
    }
}

/// What a stream event changed, for the page to show. `index` numbers the parts of
/// one response, in the order they started.
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
        /// For a tool call, its complete input: pretty-printed, or the raw text when
        /// it was not valid JSON.
        tool_input: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_production_endpoints_are_https() {
        for provider in Provider::ALL {
            assert!(production_url(provider).starts_with("https://"));
            assert!(Endpoint::production(provider).is_production(provider));
            assert!(!Endpoint::local("http://127.0.0.1:1").is_production(provider));
        }
        assert_eq!(anthropic::PRODUCTION_URL, "https://api.anthropic.com");
        assert_eq!(openai::PRODUCTION_URL, "https://api.openai.com");
        assert!(!Endpoint::production(Provider::OpenAi).is_production(Provider::Anthropic));
    }

    #[test]
    fn a_rejected_request_says_to_clear_the_conversation() {
        let key = ApiKey::parse(Provider::OpenAi, "sk-proj-test-key-0123456789abcdef").unwrap();
        let error = ApiError::api(Some(400), ErrorKind::InvalidRequest, "bad", None, &key);
        for model in Model::ALL {
            let text = error.explain(model);
            assert!(
                text.starts_with(
                    "The API rejected the request. Clear the conversation and try again."
                ),
                "{text}"
            );
            assert!(text.ends_with("(400): bad"), "{text}");
        }
    }
}
