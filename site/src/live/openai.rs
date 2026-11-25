//! The OpenAI Responses API: the request, its headers, the streamed response and its
//! errors.
//!
//! This follows the raw HTTP API: a `POST` to `/v1/responses` with `stream: true`,
//! read as server-sent events. `store` is `false`, so OpenAI keeps nothing, and the
//! whole conversation goes in `input` each time, with the encrypted reasoning items
//! sent back as they came. Shapes are copied from these pages (read 2026-11-24):
//!
//! - Create a response: https://developers.openai.com/api/reference/resources/responses/methods/create
//! - Streaming events: https://developers.openai.com/api/reference/resources/responses/streaming-events
//! - Reasoning: https://developers.openai.com/api/docs/guides/reasoning
//! - Function calling: https://developers.openai.com/api/docs/guides/function-calling
//! - Error codes: https://developers.openai.com/api/docs/guides/error-codes
//! - Authentication: https://developers.openai.com/api/reference/overview

use std::collections::{BTreeMap, HashMap};

use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{Value, json};

use super::{
    api::{ApiError, ErrorKind, MAX_TOKENS, SYSTEM_PROMPT, Update},
    key::ApiKey,
    provider::{Finished, Model, Outcome, ToolCall},
    sse, tools,
};

/// The only OpenAI host the key is ever sent to in a release build.
pub const PRODUCTION_URL: &str = "https://api.openai.com";

/// The path requests are sent to, under the endpoint's base URL.
pub const PATH: &str = "/v1/responses";

/// The header that authenticates a request: `Authorization: Bearer <key>`, marked
/// sensitive so it prints as `Sensitive` in reqwest's and http's `Debug` output. No
/// organization or project header is sent; the key picks its own project.
pub fn headers(key: &ApiKey) -> Result<HeaderMap, ApiError> {
    let mut headers = HeaderMap::new();
    let mut value = HeaderValue::from_str(&format!("Bearer {}", key.expose()))
        .map_err(|_| ApiError::Transport)?;
    value.set_sensitive(true);
    headers.insert("authorization", value);
    Ok(headers)
}

/// The request body for one step of a conversation.
pub fn request_body(model: Model, input: &[Value]) -> Value {
    json!({
        "model": model.id(),
        "instructions": SYSTEM_PROMPT,
        "input": input,
        "stream": true,
        // Nothing is kept on OpenAI's side; the history travels in `input`.
        "store": false,
        "max_output_tokens": MAX_TOKENS,
        // "auto" is the most detailed summary the model offers. Some models or
        // organizations get none, which the stream handles.
        "reasoning": { "summary": "auto" },
        // Reasoning items come back encrypted, to be sent with the next request.
        // Stateless requests get this by default now; the guide says the value is
        // still accepted, and it keeps the request explicit.
        "include": ["reasoning.encrypted_content"],
        "tools": function_tools(),
    })
}

/// The demo's tools as Responses API function tools: the same names, descriptions
/// and JSON schemas. `strict` is off because the schemas leave out
/// `additionalProperties: false`, which strict mode requires; the tools check their
/// input themselves.
fn function_tools() -> Value {
    let tools = tools::definitions()
        .as_array()
        .into_iter()
        .flatten()
        .map(|tool| {
            json!({
                "type": "function",
                "name": tool["name"],
                "description": tool["description"],
                "parameters": tool["input_schema"],
                "strict": false,
            })
        })
        .collect();
    Value::Array(tools)
}

/// A user message with `text`.
pub fn user_items(text: &str) -> Vec<Value> {
    vec![json!({ "type": "message", "role": "user", "content": text })]
}

/// The text of a reply that was stopped or failed, as an assistant message.
pub fn interrupted_items(text: &str) -> Vec<Value> {
    vec![json!({ "type": "message", "role": "assistant", "content": text })]
}

/// One `function_call_output` item per call. The output text says when it is an
/// error; the item has no error flag.
pub fn tool_result_items(results: &[(ToolCall, String, bool)]) -> Vec<Value> {
    results
        .iter()
        .map(|(call, text, _)| {
            json!({ "type": "function_call_output", "call_id": call.id, "output": text })
        })
        .collect()
}

/// The error for an error response, or with no `status`, for an `error` event's
/// data in a stream. A response carries `{"error": {"message", "type", "code"}}`;
/// an `error` event carries `code` and `message` at the top level.
pub fn parse_error(
    status: Option<u16>,
    body: &str,
    retry_after: Option<u64>,
    key: &ApiKey,
) -> ApiError {
    let parsed = serde_json::from_str::<Value>(body).ok();
    let error = parsed.as_ref().map(|body| {
        body.get("error")
            .filter(|error| error.is_object())
            .unwrap_or(body)
    });
    error_from(status, error, retry_after, key)
}

/// The error for an error object with `code`, `type` and `message`.
fn error_from(
    status: Option<u16>,
    error: Option<&Value>,
    retry_after: Option<u64>,
    key: &ApiKey,
) -> ApiError {
    let field = |name| {
        error
            .and_then(|error| error.get(name))
            .and_then(Value::as_str)
    };
    let kind = kind_for(status, field("code"), field("type"));
    ApiError::api(
        status,
        kind,
        field("message").unwrap_or_default(),
        retry_after,
        key,
    )
}

/// Codes and types that mean the account has no credit. The error codes guide says
/// to read `error.code` for billing errors, and that `error.type` can still be
/// `insufficient_quota`.
const BILLING: [&str; 2] = ["insufficient_quota", "credit_balance_exhausted"];

/// The kind for a status and the error's `code` and `type`.
fn kind_for(status: Option<u16>, code: Option<&str>, error_type: Option<&str>) -> ErrorKind {
    let names = [code, error_type];
    let named = |wanted: &[&str]| names.iter().flatten().any(|name| wanted.contains(name));
    if named(&BILLING) {
        return ErrorKind::Billing;
    }
    if named(&["model_not_found"]) {
        return ErrorKind::NotFound;
    }
    match status {
        Some(400) => ErrorKind::InvalidRequest,
        Some(401) => ErrorKind::Authentication,
        Some(403) => ErrorKind::Permission,
        Some(404) => ErrorKind::NotFound,
        Some(429) => ErrorKind::RateLimit,
        Some(503) => ErrorKind::Overloaded,
        Some(504) => ErrorKind::Timeout,
        Some(_) => ErrorKind::Api,
        // An error in a stream has no status; its code says what happened.
        None => match code.or(error_type) {
            None | Some("server_error") => ErrorKind::Api,
            Some("rate_limit_exceeded" | "rate_limit_error" | "slow_down") => ErrorKind::RateLimit,
            Some("server_is_overloaded" | "service_unavailable_error") => ErrorKind::Overloaded,
            Some("invalid_request_error") => ErrorKind::InvalidRequest,
            Some(other) => ErrorKind::Other(other.to_owned()),
        },
    }
}

/// One streamed piece of the output, which the page shows as one part: a reasoning
/// summary part, a content part of a message, or a function call. Keyed by output
/// item index and the index within the item.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum Piece {
    Summary { item: usize, summary: usize },
    Content { item: usize, content: usize },
    Call { item: usize },
}

impl Piece {
    fn item(self) -> usize {
        match self {
            Self::Summary { item, .. } | Self::Content { item, .. } | Self::Call { item } => item,
        }
    }
}

/// How the response ended.
enum End {
    Completed,
    Incomplete(Option<String>),
}

/// One response, built from the events of a stream.
///
/// The page sees each piece under a dense index, 0, 1, 2… in the order the pieces
/// started. A reasoning summary part starts only with its first text, so a
/// reasoning item with no summary shows nothing.
#[derive(Default)]
pub struct Accumulator {
    /// The finished output items by output index, from `response.output_item.done`,
    /// to send back exactly as given.
    items: BTreeMap<usize, Value>,
    /// The dense index of each piece that started.
    pieces: HashMap<Piece, usize>,
    /// Pieces started and not yet stopped, by dense index.
    open: BTreeMap<usize, Piece>,
    /// The arguments of each function call, by output index, as they stream.
    arguments: HashMap<usize, String>,
    /// A content part was a refusal.
    refused: bool,
    end: Option<End>,
}

impl Accumulator {
    /// Applies one event. An `error` or `response.failed` event becomes an
    /// [`ApiError`].
    pub fn apply(&mut self, event: &sse::Event, key: &ApiKey) -> Result<Vec<Update>, ApiError> {
        let data: Value = serde_json::from_str(&event.data).map_err(|_| ApiError::Protocol)?;
        let kind = data
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or(&event.name);
        let number = |field: &str| {
            data.get(field)
                .and_then(Value::as_u64)
                .and_then(|index| usize::try_from(index).ok())
                .ok_or(ApiError::Protocol)
        };
        let mut updates = Vec::new();
        match kind {
            "response.output_item.added" => {
                let item = number("output_index")?;
                let added = data.get("item").ok_or(ApiError::Protocol)?;
                if added["type"] == "function_call" {
                    let index = self.start(Piece::Call { item });
                    self.arguments
                        .insert(item, string_field(added, "arguments"));
                    updates.push(Update::ToolStarted {
                        index,
                        id: string_field(added, "call_id"),
                        name: string_field(added, "name"),
                    });
                }
            }
            "response.function_call_arguments.delta" => {
                let item = number("output_index")?;
                let index = *self
                    .pieces
                    .get(&Piece::Call { item })
                    .ok_or(ApiError::Protocol)?;
                let partial = string_field(&data, "delta");
                self.arguments.entry(item).or_default().push_str(&partial);
                updates.push(Update::ToolInput { index, partial });
            }
            "response.reasoning_summary_text.delta" => {
                let piece = Piece::Summary {
                    item: number("output_index")?,
                    summary: number("summary_index")?,
                };
                let text = string_field(&data, "delta");
                if !text.is_empty() {
                    let index = self.started(piece, &mut updates);
                    updates.push(Update::Thinking { index, text });
                }
            }
            "response.reasoning_summary_part.done" => {
                let piece = Piece::Summary {
                    item: number("output_index")?,
                    summary: number("summary_index")?,
                };
                // A part whose text came only with its end still shows.
                let text = data
                    .get("part")
                    .map(|part| string_field(part, "text"))
                    .unwrap_or_default();
                if !self.pieces.contains_key(&piece) && !text.is_empty() {
                    let index = self.started(piece, &mut updates);
                    updates.push(Update::Thinking { index, text });
                }
                self.stop(piece, None, &mut updates);
            }
            "response.content_part.added" => {
                let piece = Piece::Content {
                    item: number("output_index")?,
                    content: number("content_index")?,
                };
                match data.get("part").and_then(|part| part.get("type")) {
                    Some(kind) if kind == "output_text" => {
                        self.started(piece, &mut updates);
                    }
                    Some(kind) if kind == "refusal" => {
                        self.refused = true;
                        self.started(piece, &mut updates);
                    }
                    _ => {}
                }
            }
            // A refusal streams as text too: it is what the model said.
            "response.output_text.delta" | "response.refusal.delta" => {
                let piece = Piece::Content {
                    item: number("output_index")?,
                    content: number("content_index")?,
                };
                if kind == "response.refusal.delta" {
                    self.refused = true;
                }
                let index = self.started(piece, &mut updates);
                updates.push(Update::Text {
                    index,
                    text: string_field(&data, "delta"),
                });
            }
            "response.content_part.done" => {
                let piece = Piece::Content {
                    item: number("output_index")?,
                    content: number("content_index")?,
                };
                if data["part"]["type"] == "refusal" {
                    self.refused = true;
                }
                self.stop(piece, None, &mut updates);
            }
            "response.output_item.done" => {
                let item = number("output_index")?;
                let done = data.get("item").cloned().ok_or(ApiError::Protocol)?;
                let tool_input = (done["type"] == "function_call").then(|| {
                    // The finished item has the whole arguments; the deltas are a
                    // fallback.
                    let raw = done
                        .get("arguments")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .or_else(|| self.arguments.remove(&item))
                        .unwrap_or_default();
                    match parse_arguments(&raw) {
                        Ok(input) => serde_json::to_string_pretty(&input).unwrap_or_default(),
                        Err(raw) => raw,
                    }
                });
                if let Some(tool_input) = tool_input {
                    self.stop(Piece::Call { item }, Some(tool_input), &mut updates);
                }
                // Anything of this item still open ends with it.
                let open: Vec<Piece> = self
                    .open
                    .values()
                    .copied()
                    .filter(|piece| piece.item() == item)
                    .collect();
                for piece in open {
                    self.stop(piece, None, &mut updates);
                }
                self.items.insert(item, done);
            }
            "response.completed" => self.end = Some(End::Completed),
            "response.incomplete" => {
                let reason = data
                    .pointer("/response/incomplete_details/reason")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                self.end = Some(End::Incomplete(reason));
            }
            "response.failed" => {
                return Err(error_from(None, data.pointer("/response/error"), None, key));
            }
            "error" => return Err(parse_error(None, &event.data, None, key)),
            // response.created, response.in_progress, the *.done events whose
            // content the item's done event repeats, and event types added later.
            _ => {}
        }
        Ok(updates)
    }

    /// Gives `piece` the next dense index and marks it open.
    fn start(&mut self, piece: Piece) -> usize {
        let index = self.pieces.len();
        self.pieces.insert(piece, index);
        self.open.insert(index, piece);
        index
    }

    /// The dense index of `piece`, starting it first, with the matching update, if it
    /// has not started.
    fn started(&mut self, piece: Piece, updates: &mut Vec<Update>) -> usize {
        if let Some(index) = self.pieces.get(&piece) {
            return *index;
        }
        let index = self.start(piece);
        updates.push(match piece {
            Piece::Summary { .. } => Update::ThinkingStarted { index },
            // Calls start with their item's added event, never here.
            Piece::Content { .. } | Piece::Call { .. } => Update::TextStarted { index },
        });
        index
    }

    /// Ends `piece` if it is open.
    fn stop(&mut self, piece: Piece, tool_input: Option<String>, updates: &mut Vec<Update>) {
        if let Some(index) = self.pieces.get(&piece)
            && self.open.remove(index).is_some()
        {
            updates.push(Update::BlockStopped {
                index: *index,
                tool_input,
            });
        }
    }

    /// The finished response, once `response.completed` or `response.incomplete`
    /// has arrived.
    pub fn finished(&self) -> Option<Finished> {
        let end = self.end.as_ref()?;
        let calls: Vec<ToolCall> = self
            .items
            .values()
            .filter(|item| item["type"] == "function_call")
            .map(|item| ToolCall {
                id: string_field(item, "call_id"),
                name: string_field(item, "name"),
                input: parse_arguments(&string_field(item, "arguments")),
            })
            .collect();
        let outcome = match end {
            End::Incomplete(Some(reason)) if reason == "max_output_tokens" => Outcome::MaxTokens,
            End::Incomplete(Some(reason)) if reason == "content_filter" => Outcome::Refusal,
            End::Incomplete(_) => Outcome::Done,
            End::Completed if !calls.is_empty() => Outcome::ToolUse,
            End::Completed if self.refused => Outcome::Refusal,
            End::Completed => Outcome::Done,
        };
        Some(Finished {
            outcome,
            calls,
            items: self.items.values().cloned().collect(),
        })
    }
}

/// A function call's arguments: a JSON object, or `Err` with the raw text. Empty
/// arguments are an empty object.
fn parse_arguments(raw: &str) -> Result<Value, String> {
    let raw_or_empty = if raw.trim().is_empty() { "{}" } else { raw };
    match serde_json::from_str::<Value>(raw_or_empty) {
        Ok(input @ Value::Object(_)) => Ok(input),
        _ => Err(raw.to_owned()),
    }
}

fn string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::provider::Provider;

    fn key() -> ApiKey {
        ApiKey::parse(Provider::OpenAi, "sk-proj-test-key-0123456789abcdef").unwrap()
    }

    /// Feeds a stream through the parser and the accumulator.
    fn run(stream: &str) -> (Accumulator, Vec<Update>, Option<ApiError>) {
        let mut parser = sse::Parser::default();
        let mut response = Accumulator::default();
        let mut updates = Vec::new();
        for event in parser.feed(stream.as_bytes()).unwrap() {
            match response.apply(&event, &key()) {
                Ok(more) => updates.extend(more),
                Err(error) => return (response, updates, Some(error)),
            }
        }
        (response, updates, None)
    }

    /// One SSE event named after its `type`.
    fn event(data: &Value) -> String {
        format!(
            "event: {}\ndata: {data}\n\n",
            data["type"].as_str().unwrap()
        )
    }

    fn stream(events: &[Value]) -> String {
        events.iter().map(event).collect()
    }

    // The event shapes are the examples in the streaming events reference.

    fn reasoning(output_index: usize, summary: &str) -> Vec<Value> {
        let id = format!("rs_{output_index}");
        let mut events = vec![json!({
            "type": "response.output_item.added", "output_index": output_index,
            "item": {"id": id, "type": "reasoning", "summary": []}, "sequence_number": 1
        })];
        if !summary.is_empty() {
            events.extend([
                json!({"type": "response.reasoning_summary_part.added", "item_id": id,
                    "output_index": output_index, "summary_index": 0,
                    "part": {"type": "summary_text", "text": ""}, "sequence_number": 2}),
                json!({"type": "response.reasoning_summary_text.delta", "item_id": id,
                    "output_index": output_index, "summary_index": 0, "delta": summary,
                    "sequence_number": 3}),
                json!({"type": "response.reasoning_summary_text.done", "item_id": id,
                    "output_index": output_index, "summary_index": 0, "text": summary,
                    "sequence_number": 4}),
                json!({"type": "response.reasoning_summary_part.done", "item_id": id,
                    "output_index": output_index, "summary_index": 0,
                    "part": {"type": "summary_text", "text": summary}, "sequence_number": 5}),
            ]);
        }
        let summaries: Vec<Value> = if summary.is_empty() {
            Vec::new()
        } else {
            vec![json!({"type": "summary_text", "text": summary})]
        };
        events.push(json!({
            "type": "response.output_item.done", "output_index": output_index,
            "item": {"id": id, "type": "reasoning", "summary": summaries,
                "encrypted_content": "gAAAAB-encrypted"},
            "sequence_number": 6
        }));
        events
    }

    fn message(output_index: usize, text: &str) -> Vec<Value> {
        let id = format!("msg_{output_index}");
        vec![
            json!({"type": "response.output_item.added", "output_index": output_index,
                "item": {"id": id, "status": "in_progress", "type": "message",
                    "role": "assistant", "content": []}, "sequence_number": 1}),
            json!({"type": "response.content_part.added", "item_id": id,
                "output_index": output_index, "content_index": 0,
                "part": {"type": "output_text", "text": "", "annotations": [], "logprobs": []},
                "sequence_number": 2}),
            json!({"type": "response.output_text.delta", "item_id": id,
                "output_index": output_index, "content_index": 0, "delta": text,
                "sequence_number": 3, "logprobs": []}),
            json!({"type": "response.output_text.done", "item_id": id,
                "output_index": output_index, "content_index": 0, "text": text,
                "sequence_number": 4, "logprobs": []}),
            json!({"type": "response.content_part.done", "item_id": id,
                "output_index": output_index, "content_index": 0, "sequence_number": 5,
                "part": {"type": "output_text", "text": text, "annotations": [], "logprobs": []}}),
            json!({"type": "response.output_item.done", "output_index": output_index,
                "item": {"id": id, "status": "completed", "type": "message",
                    "role": "assistant", "content": [{"type": "output_text", "text": text,
                        "annotations": [], "logprobs": []}]}, "sequence_number": 6}),
        ]
    }

    fn function_call(output_index: usize, arguments: &str) -> Vec<Value> {
        let (first, rest) = arguments.split_at(arguments.len() / 2);
        let item = json!({"id": "fc_1234xyz", "type": "function_call", "call_id": "call_2345abc",
            "name": "calculate", "arguments": arguments, "status": "completed"});
        vec![
            json!({"type": "response.output_item.added", "output_index": output_index,
                "item": {"id": "fc_1234xyz", "type": "function_call", "call_id": "call_2345abc",
                    "name": "calculate", "arguments": "", "status": "in_progress"},
                "sequence_number": 1}),
            json!({"type": "response.function_call_arguments.delta", "item_id": "fc_1234xyz",
                "output_index": output_index, "delta": first, "sequence_number": 2}),
            json!({"type": "response.function_call_arguments.delta", "item_id": "fc_1234xyz",
                "output_index": output_index, "delta": rest, "sequence_number": 3}),
            json!({"type": "response.function_call_arguments.done", "item_id": "fc_1234xyz",
                "output_index": output_index, "arguments": arguments, "sequence_number": 4}),
            json!({"type": "response.output_item.done", "output_index": output_index,
                "item": item, "sequence_number": 5}),
        ]
    }

    fn completed() -> Value {
        json!({"type": "response.completed", "response": {"id": "resp_123",
            "object": "response", "status": "completed", "error": null,
            "incomplete_details": null, "output": []}, "sequence_number": 9})
    }

    #[test]
    fn accumulates_reasoning_and_text_with_dense_indexes() {
        let mut events = vec![json!({"type": "response.created", "response": {
            "id": "resp_1", "object": "response", "status": "in_progress", "output": []},
            "sequence_number": 0})];
        events.extend(reasoning(0, "**Answering**\n\nParis."));
        events.extend(message(1, "The capital of France is Paris."));
        events.push(completed());
        let (response, updates, error) = run(&stream(&events));
        assert_eq!(error, None);
        assert_eq!(
            updates,
            [
                Update::ThinkingStarted { index: 0 },
                Update::Thinking {
                    index: 0,
                    text: "**Answering**\n\nParis.".to_owned()
                },
                Update::BlockStopped {
                    index: 0,
                    tool_input: None
                },
                Update::TextStarted { index: 1 },
                Update::Text {
                    index: 1,
                    text: "The capital of France is Paris.".to_owned()
                },
                Update::BlockStopped {
                    index: 1,
                    tool_input: None
                },
            ]
        );
        let finished = response.finished().unwrap();
        assert_eq!(finished.outcome, Outcome::Done);
        assert!(finished.calls.is_empty());
        // The items go back exactly as the done events gave them.
        assert_eq!(finished.items.len(), 2);
        assert_eq!(finished.items[0]["encrypted_content"], "gAAAAB-encrypted");
        assert_eq!(
            finished.items[1]["content"][0]["text"],
            "The capital of France is Paris."
        );
    }

    #[test]
    fn a_reasoning_item_without_summary_shows_nothing() {
        let mut events = reasoning(0, "");
        // A summary part that ends empty does not show either.
        events.insert(
            1,
            json!({"type": "response.reasoning_summary_part.added", "item_id": "rs_0",
                "output_index": 0, "summary_index": 0,
                "part": {"type": "summary_text", "text": ""}, "sequence_number": 2}),
        );
        events.insert(
            2,
            json!({"type": "response.reasoning_summary_part.done", "item_id": "rs_0",
                "output_index": 0, "summary_index": 0,
                "part": {"type": "summary_text", "text": ""}, "sequence_number": 3}),
        );
        events.extend(message(1, "Hi."));
        events.push(completed());
        let (response, updates, error) = run(&stream(&events));
        assert_eq!(error, None);
        assert!(
            !updates
                .iter()
                .any(|update| matches!(update, Update::ThinkingStarted { .. }))
        );
        // The text is the first piece the page sees.
        assert_eq!(updates[0], Update::TextStarted { index: 0 });
        // The reasoning item still goes back.
        assert_eq!(response.finished().unwrap().items.len(), 2);
    }

    #[test]
    fn accumulates_a_function_call() {
        let mut events = reasoning(0, "Use the calculator.");
        events.extend(message(1, "Let me work that out."));
        events.extend(function_call(2, "{\"expression\":\"2+2\"}"));
        events.push(completed());
        let (response, updates, error) = run(&stream(&events));
        assert_eq!(error, None);
        assert!(updates.contains(&Update::ToolStarted {
            index: 2,
            id: "call_2345abc".to_owned(),
            name: "calculate".to_owned(),
        }));
        assert!(updates.contains(&Update::ToolInput {
            index: 2,
            partial: "{\"expressi".to_owned()
        }));
        let pretty = serde_json::to_string_pretty(&json!({"expression": "2+2"})).unwrap();
        assert_eq!(
            updates.last(),
            Some(&Update::BlockStopped {
                index: 2,
                tool_input: Some(pretty)
            })
        );
        let finished = response.finished().unwrap();
        assert_eq!(finished.outcome, Outcome::ToolUse);
        assert_eq!(
            finished.calls,
            [ToolCall {
                id: "call_2345abc".to_owned(),
                name: "calculate".to_owned(),
                input: Ok(json!({"expression": "2+2"})),
            }]
        );
        assert_eq!(
            finished
                .items
                .iter()
                .map(|item| item["type"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["reasoning", "message", "function_call"]
        );
    }

    #[test]
    fn keeps_invalid_arguments_aside() {
        let mut events = function_call(0, "{\"expression\": \"1+");
        events.push(completed());
        let (response, updates, error) = run(&stream(&events));
        assert_eq!(error, None);
        assert_eq!(
            updates.last(),
            Some(&Update::BlockStopped {
                index: 0,
                tool_input: Some("{\"expression\": \"1+".to_owned())
            })
        );
        let finished = response.finished().unwrap();
        assert_eq!(
            finished.calls[0].input,
            Err("{\"expression\": \"1+".to_owned())
        );
        // The item goes back as the API returned it.
        assert_eq!(finished.items[0]["arguments"], "{\"expression\": \"1+");
    }

    #[test]
    fn nothing_is_finished_before_the_terminal_event() {
        let (response, _, error) = run(&stream(&message(0, "Hi")));
        assert_eq!(error, None);
        assert!(response.finished().is_none());
    }

    #[test]
    fn incomplete_and_refused_responses() {
        let mut events = message(0, "A long answer");
        events.push(
            json!({"type": "response.incomplete", "response": {"id": "resp_123",
            "status": "incomplete", "incomplete_details": {"reason": "max_output_tokens"},
            "output": []}, "sequence_number": 7}),
        );
        let (response, _, _) = run(&stream(&events));
        assert_eq!(response.finished().unwrap().outcome, Outcome::MaxTokens);

        let refusal = vec![
            json!({"type": "response.output_item.added", "output_index": 0,
                "item": {"id": "msg_1", "status": "in_progress", "type": "message",
                    "role": "assistant", "content": []}, "sequence_number": 1}),
            json!({"type": "response.content_part.added", "item_id": "msg_1",
                "output_index": 0, "content_index": 0,
                "part": {"type": "refusal", "refusal": ""}, "sequence_number": 2}),
            json!({"type": "response.refusal.delta", "item_id": "msg_1", "output_index": 0,
                "content_index": 0, "delta": "I can't help with that.", "sequence_number": 3}),
            json!({"type": "response.refusal.done", "item_id": "msg_1", "output_index": 0,
                "content_index": 0, "refusal": "I can't help with that.", "sequence_number": 4}),
            json!({"type": "response.content_part.done", "item_id": "msg_1",
                "output_index": 0, "content_index": 0,
                "part": {"type": "refusal", "refusal": "I can't help with that."},
                "sequence_number": 5}),
            completed(),
        ];
        let (response, updates, error) = run(&stream(&refusal));
        assert_eq!(error, None);
        assert!(updates.contains(&Update::Text {
            index: 0,
            text: "I can't help with that.".to_owned()
        }));
        assert_eq!(response.finished().unwrap().outcome, Outcome::Refusal);
    }

    #[test]
    fn error_and_failed_events_end_the_stream() {
        let mut events = message(0, "Hel");
        events.truncate(3);
        events.push(json!({"type": "error", "code": "server_is_overloaded",
            "message": "The model is overloaded.", "param": null, "sequence_number": 4}));
        let (_, updates, error) = run(&stream(&events));
        assert!(updates.contains(&Update::Text {
            index: 0,
            text: "Hel".to_owned()
        }));
        assert_eq!(
            error,
            Some(ApiError::Api {
                status: None,
                kind: ErrorKind::Overloaded,
                message: "The model is overloaded.".to_owned(),
                retry_after: None,
            })
        );

        let failed = json!({"type": "response.failed", "response": {"id": "resp_123",
            "object": "response", "status": "failed",
            "error": {"code": "server_error", "message": "The model failed to generate a response."},
            "output": []}, "sequence_number": 1});
        let (_, _, error) = run(&event(&failed));
        assert_eq!(
            error,
            Some(ApiError::Api {
                status: None,
                kind: ErrorKind::Api,
                message: "The model failed to generate a response.".to_owned(),
                retry_after: None,
            })
        );
    }

    #[test]
    fn unknown_events_are_ignored() {
        let (_, updates, error) = run(&stream(&[
            json!({"type": "response.something_new", "sequence_number": 1}),
            json!({"type": "response.web_search_call.in_progress", "output_index": 0,
                "item_id": "ws_1", "sequence_number": 2}),
        ]));
        assert_eq!(error, None);
        assert!(updates.is_empty());
    }

    #[test]
    fn maps_the_documented_errors() {
        let key = key();
        let cases = [
            (400, None, None, ErrorKind::InvalidRequest),
            (401, None, None, ErrorKind::Authentication),
            (403, None, None, ErrorKind::Permission),
            (
                404,
                Some("model_not_found"),
                Some("invalid_request_error"),
                ErrorKind::NotFound,
            ),
            (400, Some("model_not_found"), None, ErrorKind::NotFound),
            (
                429,
                Some("insufficient_quota"),
                Some("insufficient_quota"),
                ErrorKind::Billing,
            ),
            (
                429,
                Some("credit_balance_exhausted"),
                None,
                ErrorKind::Billing,
            ),
            (429, None, Some("insufficient_quota"), ErrorKind::Billing),
            (
                429,
                Some("slow_down"),
                Some("rate_limit_error"),
                ErrorKind::RateLimit,
            ),
            (
                429,
                Some("project_spend_limit_exceeded"),
                None,
                ErrorKind::RateLimit,
            ),
            (500, None, Some("server_error"), ErrorKind::Api),
            (
                503,
                Some("server_is_overloaded"),
                Some("service_unavailable_error"),
                ErrorKind::Overloaded,
            ),
            (504, None, None, ErrorKind::Timeout),
        ];
        for (status, code, error_type, kind) in cases {
            let body = json!({"error": {"message": "m", "type": error_type, "param": null,
                "code": code}})
            .to_string();
            assert_eq!(
                parse_error(Some(status), &body, None, &key),
                ApiError::Api {
                    status: Some(status),
                    kind: kind.clone(),
                    message: "m".to_owned(),
                    retry_after: None,
                },
                "{status} {code:?}"
            );
        }
        // A body that is not JSON falls back to the status.
        let ApiError::Api { kind, .. } = parse_error(Some(503), "<html>", None, &key) else {
            panic!("an API error");
        };
        assert_eq!(kind, ErrorKind::Overloaded);
        let billing = parse_error(
            Some(429),
            &json!({"error": {"message": "m", "type": "insufficient_quota",
                "code": "insufficient_quota"}})
            .to_string(),
            None,
            &key,
        );
        assert!(billing.explain(Model::Sol).contains("billing problem"));
        assert!(!billing.explain(Model::Sol).contains("rate limit"));
    }

    #[test]
    fn error_messages_never_carry_the_key() {
        let key = key();
        let tail = &key.expose()[key.expose().len() - 20..];
        let body = json!({"error": {"message": format!(
            "Incorrect API key provided: {}. Key ending {tail}.", key.expose()),
            "type": "invalid_request_error", "code": "invalid_api_key"}})
        .to_string();
        let error = parse_error(Some(401), &body, None, &key);
        for printed in [
            error.explain(Model::Sol),
            format!("{error}"),
            format!("{error:?}"),
        ] {
            assert!(!printed.contains(key.expose()), "{printed}");
            assert!(!printed.contains(tail), "{printed}");
        }
        assert!(error.is_key_problem());
    }

    #[test]
    fn request_body_follows_the_docs() {
        let body = request_body(Model::Sol, &user_items("hi"));
        assert_eq!(body["model"], "gpt-6.2-sol");
        assert_eq!(body["instructions"], SYSTEM_PROMPT);
        assert_eq!(body["max_output_tokens"], MAX_TOKENS);
        assert_eq!(body["stream"], true);
        assert_eq!(body["store"], false);
        assert_eq!(body["reasoning"], json!({"summary": "auto"}));
        assert_eq!(body["include"], json!(["reasoning.encrypted_content"]));
        assert_eq!(
            body["input"],
            json!([{"type": "message", "role": "user", "content": "hi"}])
        );
        let definitions = tools::definitions();
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), definitions.as_array().unwrap().len());
        for (tool, definition) in tools.iter().zip(definitions.as_array().unwrap()) {
            assert_eq!(tool["type"], "function");
            assert_eq!(tool["name"], definition["name"]);
            assert_eq!(tool["description"], definition["description"]);
            assert_eq!(tool["parameters"], definition["input_schema"]);
            assert_eq!(tool["strict"], false);
        }
    }

    #[test]
    fn tool_results_are_function_call_outputs() {
        let call = ToolCall {
            id: "call_1".to_owned(),
            name: "calculate".to_owned(),
            input: Ok(json!({})),
        };
        assert_eq!(
            tool_result_items(&[(call, "INVALID_JSON: no".to_owned(), true)]),
            [json!({"type": "function_call_output", "call_id": "call_1",
                "output": "INVALID_JSON: no"})]
        );
        assert_eq!(
            interrupted_items("part"),
            [json!({"type": "message", "role": "assistant", "content": "part"})]
        );
    }

    #[test]
    fn the_key_goes_in_a_sensitive_bearer_header() {
        let key = key();
        let headers = headers(&key).unwrap();
        let value = &headers["authorization"];
        assert!(value.is_sensitive());
        assert_eq!(value.to_str().unwrap(), format!("Bearer {}", key.expose()));
        assert!(!format!("{headers:?}").contains(key.expose()));
        assert_eq!(headers.len(), 1);
    }
}
