//! The Anthropic Messages API: the request, its headers, the streamed response and
//! its errors.
//!
//! There is no official Rust SDK, so this follows the raw HTTP API: a `POST` to
//! `/v1/messages` with `stream: true`, read as server-sent events. Event and error
//! shapes follow the Claude API docs ("Streaming messages" and "Claude API errors").

use std::collections::HashMap;

use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{Value, json};

use super::{
    api::{ApiError, ErrorKind, MAX_TOKENS, SYSTEM_PROMPT, Update},
    key::ApiKey,
    provider::{Finished, Model, Outcome, ToolCall},
    sse, tools,
};

/// The API version the requests and the parsing are written against.
pub const API_VERSION: &str = "2026-10-21";

/// The only Anthropic host the key is ever sent to in a release build.
pub const PRODUCTION_URL: &str = "https://api.anthropic.com";

/// The path requests are sent to, under the endpoint's base URL.
pub const PATH: &str = "/v1/messages";

/// The headers that authenticate a request and pick the API version. The key's
/// header is marked sensitive, so it prints as `Sensitive` in reqwest's and http's
/// `Debug` output.
pub fn headers(key: &ApiKey) -> Result<HeaderMap, ApiError> {
    let mut headers = HeaderMap::new();
    let mut key_header = HeaderValue::from_str(key.expose()).map_err(|_| ApiError::Transport)?;
    key_header.set_sensitive(true);
    headers.insert("x-api-key", key_header);
    headers.insert("anthropic-version", HeaderValue::from_static(API_VERSION));
    Ok(headers)
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

/// A user message with `text`.
pub fn user_items(text: &str) -> Vec<Value> {
    vec![json!({ "role": "user", "content": [{ "type": "text", "text": text }] })]
}

/// The text of a reply that was stopped or failed, as an assistant message.
pub fn interrupted_items(text: &str) -> Vec<Value> {
    vec![json!({ "role": "assistant", "content": [{ "type": "text", "text": text }] })]
}

/// The results of a round's tool calls, all in one user message.
pub fn tool_result_items(results: &[(ToolCall, String, bool)]) -> Vec<Value> {
    let content: Vec<Value> = results
        .iter()
        .map(|(call, text, is_error)| {
            let mut result =
                json!({ "type": "tool_result", "tool_use_id": call.id, "content": text });
            if *is_error {
                result["is_error"] = Value::Bool(true);
            }
            result
        })
        .collect();
    vec![json!({ "role": "user", "content": content })]
}

/// The history as the API takes it. A reply that ended before the model wrote
/// anything, or after tool results, leaves two user messages in a row; they are
/// merged, results first, as the API requires.
pub fn prepare_history(items: Vec<Value>) -> Vec<Value> {
    let mut messages: Vec<Value> = Vec::new();
    for message in items {
        push_merged(&mut messages, message);
    }
    messages
}

fn push_merged(messages: &mut Vec<Value>, message: Value) {
    if let Some(last) = messages.last_mut()
        && last["role"] == "user"
        && message["role"] == "user"
        && let (Some(content), Some(more)) = (
            last["content"].as_array_mut(),
            message["content"].as_array(),
        )
    {
        content.extend(more.iter().cloned());
        return;
    }
    messages.push(message);
}

/// The error for an error response, or with no `status`, for the data of an
/// `error` event in a stream. Both carry `{"error": {"type", "message"}}`.
pub fn parse_error(
    status: Option<u16>,
    body: &str,
    retry_after: Option<u64>,
    key: &ApiKey,
) -> ApiError {
    let parsed = serde_json::from_str::<Value>(body).ok();
    let error = parsed.as_ref().and_then(|body| body.get("error"));
    let kind = error
        .and_then(|error| error.get("type"))
        .and_then(Value::as_str)
        .map_or_else(
            || status.map_or(ErrorKind::Api, kind_from_status),
            kind_from_type,
        );
    let message = error
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    ApiError::api(status, kind, message, retry_after, key)
}

/// The kind for a documented `error.type`.
fn kind_from_type(error_type: &str) -> ErrorKind {
    match error_type {
        "invalid_request_error" => ErrorKind::InvalidRequest,
        "authentication_error" => ErrorKind::Authentication,
        "billing_error" => ErrorKind::Billing,
        "permission_error" => ErrorKind::Permission,
        "not_found_error" => ErrorKind::NotFound,
        "request_too_large" => ErrorKind::RequestTooLarge,
        "rate_limit_error" => ErrorKind::RateLimit,
        "api_error" => ErrorKind::Api,
        "timeout_error" => ErrorKind::Timeout,
        "overloaded_error" => ErrorKind::Overloaded,
        other => ErrorKind::Other(other.to_owned()),
    }
}

/// The kind for a status whose body had no error type.
fn kind_from_status(status: u16) -> ErrorKind {
    match status {
        400 => ErrorKind::InvalidRequest,
        401 => ErrorKind::Authentication,
        402 => ErrorKind::Billing,
        403 => ErrorKind::Permission,
        404 => ErrorKind::NotFound,
        413 => ErrorKind::RequestTooLarge,
        429 => ErrorKind::RateLimit,
        504 => ErrorKind::Timeout,
        529 => ErrorKind::Overloaded,
        _ => ErrorKind::Api,
    }
}

/// One assistant message, built from the events of a stream.
#[derive(Default)]
pub struct Accumulator {
    /// The content blocks as the API sent them, so they go back unchanged; thinking
    /// blocks in particular must be returned exactly.
    content: Vec<Value>,
    /// The raw input of each tool-use block, by index, until the block stops.
    inputs: HashMap<usize, String>,
    /// Tool-use inputs that were not valid JSON, by index: the raw text.
    invalid_inputs: HashMap<usize, String>,
    stop_reason: Option<String>,
    /// `message_stop` arrived.
    stopped: bool,
}

impl Accumulator {
    /// Applies one event. An `error` event becomes an [`ApiError`].
    pub fn apply(&mut self, event: &sse::Event, key: &ApiKey) -> Result<Option<Update>, ApiError> {
        let data: Value = serde_json::from_str(&event.data).map_err(|_| ApiError::Protocol)?;
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
                let index = index.ok_or(ApiError::Protocol)?;
                let block = data
                    .get("content_block")
                    .cloned()
                    .ok_or(ApiError::Protocol)?;
                // Indexes count up from 0; a gap would mean a lost event.
                if index != self.content.len() {
                    return Err(ApiError::Protocol);
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
                let index = index.ok_or(ApiError::Protocol)?;
                let delta = data.get("delta").ok_or(ApiError::Protocol)?;
                let block = self.content.get_mut(index).ok_or(ApiError::Protocol)?;
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
                let index = index.ok_or(ApiError::Protocol)?;
                let mut tool_input = None;
                if let Some(raw) = self.inputs.remove(&index) {
                    // The final input is always an object; an empty one streams no
                    // fragments at all.
                    let raw_or_empty = if raw.trim().is_empty() { "{}" } else { &raw };
                    let input = match serde_json::from_str::<Value>(raw_or_empty) {
                        Ok(input @ Value::Object(_)) => {
                            tool_input =
                                Some(serde_json::to_string_pretty(&input).unwrap_or_default());
                            input
                        }
                        _ => {
                            tool_input = Some(raw.clone());
                            self.invalid_inputs.insert(index, raw);
                            json!({})
                        }
                    };
                    let block = self.content.get_mut(index).ok_or(ApiError::Protocol)?;
                    block["input"] = input;
                }
                Some(Update::BlockStopped { index, tool_input })
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
                self.stopped = true;
                None
            }
            "error" => return Err(parse_error(None, &event.data, None, key)),
            // message_start, ping, and event types added later.
            _ => None,
        })
    }

    /// The finished message, once `message_stop` has arrived.
    pub fn finished(&self) -> Option<Finished> {
        if !self.stopped {
            return None;
        }
        let outcome = match self.stop_reason.as_deref() {
            Some("tool_use") => Outcome::ToolUse,
            Some("max_tokens") => Outcome::MaxTokens,
            Some("refusal") => Outcome::Refusal,
            // end_turn, stop_sequence, and reasons added later.
            _ => Outcome::Done,
        };
        let calls = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, block)| block["type"] == "tool_use")
            .map(|(index, block)| ToolCall {
                id: string_field(block, "id"),
                name: string_field(block, "name"),
                input: match self.invalid_inputs.get(&index) {
                    Some(raw) => Err(raw.clone()),
                    None => Ok(block["input"].clone()),
                },
            })
            .collect();
        Some(Finished {
            outcome,
            calls,
            items: vec![self.assistant_message()],
        })
    }

    /// The message to send back as the assistant's turn. Empty text blocks are left
    /// out, since the API rejects them; every other block goes back as it came.
    fn assistant_message(&self) -> Value {
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
    use crate::live::provider::Provider;

    fn key() -> ApiKey {
        ApiKey::parse(
            Provider::Anthropic,
            "sk-ant-api03-test-key-0123456789abcdef",
        )
        .unwrap()
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
        assert!(message.stopped);
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
        let finished = message.finished().unwrap();
        assert_eq!(finished.outcome, Outcome::ToolUse);
        assert_eq!(
            finished.calls,
            [ToolCall {
                id: "toolu_01T1x1fJ34qAmk2tNTrN7Up6".to_owned(),
                name: "get_weather".to_owned(),
                input: Ok(json!({ "location": "San Francisco, CA" })),
            }]
        );
        assert_eq!(finished.items, [message.assistant_message()]);
        assert!(updates.contains(&Update::ToolStarted {
            index: 1,
            id: "toolu_01T1x1fJ34qAmk2tNTrN7Up6".to_owned(),
            name: "get_weather".to_owned(),
        }));
    }

    #[test]
    fn keeps_invalid_tool_input_aside() {
        let (message, updates, error) = run(
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"t\",\"name\":\"calculate\",\"input\":{}}}\n\n\
             data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"expression\\\": \\\"1+\"}}\n\n\
             data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        );
        assert_eq!(error, None);
        assert_eq!(message.content[0]["input"], json!({}));
        assert_eq!(message.invalid_inputs[&0], "{\"expression\": \"1+");
        assert_eq!(
            updates.last(),
            Some(&Update::BlockStopped {
                index: 0,
                tool_input: Some("{\"expression\": \"1+".to_owned())
            })
        );
        // No message_stop, so the message is not finished.
        assert!(message.finished().is_none());
    }

    #[test]
    fn tool_input_arrives_with_block_stop() {
        let (_, updates, error) = run(
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"t\",\"name\":\"calculate\",\"input\":{}}}\n\n\
             data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"expression\\\":\"}}\n\n\
             data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\" \\\"2+2\\\"}\"}}\n\n\
             data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        );
        assert_eq!(error, None);
        let pretty = serde_json::to_string_pretty(&json!({"expression": "2+2"})).unwrap();
        assert_eq!(
            updates.last(),
            Some(&Update::BlockStopped {
                index: 0,
                tool_input: Some(pretty)
            })
        );
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
        assert_eq!(
            updates,
            [Update::BlockStopped {
                index: 0,
                tool_input: None
            }]
        );
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
            let error = parse_error(Some(status), &body, None, &key);
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
                parse_error(Some(status), "<html>", None, &key)
            else {
                panic!("an API error");
            };
            if status != 500 {
                assert_eq!(fallback, kind, "{status}");
            }
        }
        let error = parse_error(Some(429), "{}", Some(30), &key);
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
        let error = parse_error(Some(401), &body, None, &key);
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
    fn history_merges_consecutive_user_messages() {
        let mut items = user_items("first");
        items.extend(interrupted_items("partial"));
        items.extend(user_items("second"));
        items.extend(user_items("third"));
        let history = prepare_history(items);
        assert_eq!(history.len(), 3);
        assert_eq!(history[2]["content"].as_array().unwrap().len(), 2);
    }
}
