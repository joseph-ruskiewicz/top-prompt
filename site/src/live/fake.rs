//! A stand-in for the Anthropic Messages API and the OpenAI Responses API, for tests
//! and for trying the demo locally without a key. Compiled only in tests and debug
//! builds.
//!
//! It speaks just enough HTTP/1.1 for one request per connection and picks the
//! protocol by path: `/v1/messages` answers with the event and error shapes from the
//! Claude API docs, `/v1/responses` with those from OpenAI's Responses API reference
//! (see [`super::openai`] for the pages). The reply depends on the last user message:
//!
//! - after tool results: thinking, then text quoting the first result;
//! - `[401]`, `[402]`, `[429]`, `[500]`, `[529]`: that error response (OpenAI:
//!   `[402]` is a 429 with code `insufficient_quota`, and `[503]` replaces `[529]`);
//! - `[midstream]`: some text, then an `overloaded_error` event;
//! - `[slow]`: text that streams for several seconds, to try stopping;
//! - `[refusal]` and `[maxtokens]`: text that ends with that stop reason;
//! - `[text]`: thinking and a Markdown answer, with no tool call;
//! - anything else: thinking, a sentence, and a tool call: `search_components` when
//!   the message mentions a component, `calculate` otherwise.
//!
//! OpenAI only:
//!
//! - `[nothink]`: a reasoning item with no summary, then text;
//! - `[badargs]`: a `calculate` call whose arguments are not valid JSON;
//! - `[echokey]`: a 401 whose message repeats the request's key.
//!
//! The OpenAI mode also checks the request: `store: false`, and well-formed `input`
//! items where every function call has its output. Anything else is a 400, so tests
//! notice a malformed history.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::sleep,
};

use super::{anthropic, openai};

/// A running fake API.
pub struct FakeApi {
    url: String,
    #[cfg_attr(not(test), allow(dead_code))]
    requests: Arc<Mutex<Vec<Recorded>>>,
}

/// A request the fake received. Deliberately not `Debug`: it holds the key.
#[derive(Clone)]
pub struct Recorded {
    /// The `x-api-key` header (Anthropic).
    pub api_key: String,
    /// The `authorization` header (OpenAI).
    pub authorization: String,
    pub version: String,
    pub path: String,
    pub body: Value,
}

impl FakeApi {
    /// Starts the fake on a free port of 127.0.0.1.
    pub async fn start() -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let requests: Arc<Mutex<Vec<Recorded>>> = Arc::default();
        let recorded = requests.clone();
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let recorded = recorded.clone();
                tokio::spawn(async move {
                    let _ = serve(socket, &recorded).await;
                });
            }
        });
        Ok(Self { url, requests })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Every request received so far.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }
}

async fn serve(mut socket: TcpStream, recorded: &Mutex<Vec<Recorded>>) -> std::io::Result<()> {
    let Some(request) = read_request(&mut socket).await? else {
        return Ok(());
    };
    recorded.lock().unwrap().push(request.clone());

    match request.path.as_str() {
        anthropic::PATH => serve_anthropic(&mut socket, &request).await,
        openai::PATH => responses::serve(&mut socket, &request).await,
        _ => write_error(&mut socket, 404, "not_found_error", "Not found", None).await,
    }
}

async fn serve_anthropic(socket: &mut TcpStream, request: &Recorded) -> std::io::Result<()> {
    if !request.api_key.starts_with("sk-ant-") {
        return write_error(
            socket,
            401,
            "authentication_error",
            "invalid x-api-key",
            None,
        )
        .await;
    }
    if request.version != anthropic::API_VERSION {
        return write_error(
            socket,
            400,
            "invalid_request_error",
            "anthropic-version: unsupported version",
            None,
        )
        .await;
    }
    match script(&request.body) {
        Script::Error {
            status,
            error_type,
            message,
            retry_after,
        } => write_error(socket, status, error_type, message, retry_after).await,
        Script::Stream(events) => write_stream(socket, events).await,
    }
}

/// Writes a 200 event stream of (delay in milliseconds, name, data) events.
async fn write_stream(
    socket: &mut TcpStream,
    events: Vec<(u64, &'static str, String)>,
) -> std::io::Result<()> {
    socket
        .write_all(
            b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\
              cache-control: no-cache\r\nconnection: close\r\n\r\n",
        )
        .await?;
    for (delay, name, data) in events {
        sleep(Duration::from_millis(delay)).await;
        socket
            .write_all(format!("event: {name}\ndata: {data}\n\n").as_bytes())
            .await?;
        socket.flush().await?;
    }
    socket.shutdown().await
}

/// Reads one request: headers, then a body of `content-length` bytes.
async fn read_request(socket: &mut TcpStream) -> std::io::Result<Option<Recorded>> {
    let mut buffer = Vec::new();
    let mut chunk = [0; 8192];
    let header_end = loop {
        if let Some(end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break end;
        }
        let read = socket.read(&mut chunk).await?;
        if read == 0 || buffer.len() > 64 * 1024 {
            return Ok(None);
        }
        buffer.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
    let mut lines = head.split("\r\n");
    let path = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .unwrap_or_default()
        .to_owned();
    let mut length = 0;
    let mut api_key = String::new();
    let mut authorization = String::new();
    let mut version = String::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match name.trim().to_ascii_lowercase().as_str() {
            "content-length" => length = value.parse().unwrap_or(0),
            "x-api-key" => api_key = value.to_owned(),
            "authorization" => authorization = value.to_owned(),
            "anthropic-version" => version = value.to_owned(),
            _ => {}
        }
    }
    let mut body = buffer[header_end + 4..].to_vec();
    while body.len() < length.min(4 * 1024 * 1024) {
        let read = socket.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    Ok(Some(Recorded {
        api_key,
        authorization,
        version,
        path,
        body: serde_json::from_slice(&body).unwrap_or(Value::Null),
    }))
}

async fn write_error(
    socket: &mut TcpStream,
    status: u16,
    error_type: &str,
    message: &str,
    retry_after: Option<u64>,
) -> std::io::Result<()> {
    let body = json!({
        "type": "error",
        "error": { "type": error_type, "message": message },
        "request_id": "req_fake",
    });
    write_json(socket, status, &body, retry_after).await
}

/// Writes an error response with a JSON body.
async fn write_json(
    socket: &mut TcpStream,
    status: u16,
    body: &Value,
    retry_after: Option<u64>,
) -> std::io::Result<()> {
    let body = body.to_string();
    let retry = retry_after
        .map(|seconds| format!("retry-after: {seconds}\r\n"))
        .unwrap_or_default();
    socket
        .write_all(
            format!(
                "HTTP/1.1 {status} Error\r\ncontent-type: application/json\r\n\
                 content-length: {}\r\nconnection: close\r\n{retry}\r\n{body}",
                body.len()
            )
            .as_bytes(),
        )
        .await?;
    socket.shutdown().await
}

enum Script {
    Error {
        status: u16,
        error_type: &'static str,
        message: &'static str,
        retry_after: Option<u64>,
    },
    /// Events as (delay in milliseconds, name, data).
    Stream(Vec<(u64, &'static str, String)>),
}

fn script(body: &Value) -> Script {
    let last = body["messages"]
        .as_array()
        .and_then(|messages| messages.last())
        .cloned()
        .unwrap_or(Value::Null);
    let blocks = last["content"].as_array().cloned().unwrap_or_default();
    let tool_result = blocks
        .iter()
        .find(|block| block["type"] == "tool_result")
        .map(|block| block["content"].as_str().unwrap_or_default().to_owned());
    let text: String = blocks
        .iter()
        .filter_map(|block| block["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let error = |status, error_type, message, retry_after| Script::Error {
        status,
        error_type,
        message,
        retry_after,
    };
    if tool_result.is_none() {
        if text.contains("[401]") {
            return error(401, "authentication_error", "invalid x-api-key", None);
        }
        if text.contains("[402]") {
            return error(
                402,
                "billing_error",
                "Your credit balance is too low to access the Claude API.",
                None,
            );
        }
        if text.contains("[429]") {
            return error(
                429,
                "rate_limit_error",
                "Number of requests has exceeded your rate limit.",
                Some(7),
            );
        }
        if text.contains("[500]") {
            return error(500, "api_error", "Internal server error", None);
        }
        if text.contains("[529]") {
            return error(529, "overloaded_error", "Overloaded", None);
        }
    }

    let mut stream = Stream::default();
    if let Some(result) = tool_result {
        stream.thinking("The tool answered, so I can reply with its result.");
        // A one-line summary: the calculator's number, or the size of a search result.
        let summary = if result.lines().count() == 1 {
            format!("`{result}`")
        } else {
            format!("{} lines", result.lines().count())
        };
        stream.text(&format!(
            "The tool returned {summary}. That answers the question."
        ));
        stream.end("end_turn");
    } else if text.contains("[midstream]") {
        stream.text("This reply starts, and then");
        stream.events.push((
            60,
            "error",
            json!({"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}})
                .to_string(),
        ));
    } else if text.contains("[slow]") {
        stream.slow_text(
            &"This reply streams slowly so there is time to stop it. ".repeat(4),
            150,
        );
        stream.end("end_turn");
    } else if text.contains("[refusal]") {
        stream.text("I can't");
        stream.end("refusal");
    } else if text.contains("[maxtokens]") {
        stream.text("A very long answer that runs out of room");
        stream.end("max_tokens");
    } else if text.contains("[text]") {
        stream.thinking("A plain answer with some code will do.");
        stream.text(
            "Here is a **Markdown** answer from the fake API.\n\n```rust\nfn main() {\n    \
             println!(\"hello\");\n}\n```\n\n- one\n- two",
        );
        stream.end("end_turn");
    } else if text.to_lowercase().contains("component") {
        stream.thinking("The question is about components; I'll search the catalog.");
        stream.text("Let me search the component catalog.");
        stream.tool("search_components", &json!({ "query": "reasoning" }));
        stream.end("tool_use");
    } else {
        let expression = expression_in(&text).unwrap_or_else(|| "6 * 7".to_owned());
        stream.thinking(&format!(
            "The user wants a number. I'll use the calculate tool on `{expression}`."
        ));
        stream.text("Let me work that out.");
        stream.tool("calculate", &json!({ "expression": expression }));
        stream.end("tool_use");
    }
    Script::Stream(stream.finish())
}

/// The longest run of arithmetic characters in `text` that has a digit.
fn expression_in(text: &str) -> Option<String> {
    text.split(|c: char| !(c.is_ascii_digit() || " +-*/%^().".contains(c)))
        .map(str::trim)
        .filter(|run| run.chars().any(|c| c.is_ascii_digit()))
        .max_by_key(|run| run.len())
        .map(str::to_owned)
}

/// Builds a stream's events with the documented shapes.
#[derive(Default)]
struct Stream {
    events: Vec<(u64, &'static str, String)>,
    index: usize,
}

impl Stream {
    fn block(&mut self, start: Value, deltas: Vec<(u64, Value)>) {
        let index = self.index;
        self.index += 1;
        self.events.push((
            20,
            "content_block_start",
            json!({"type": "content_block_start", "index": index, "content_block": start})
                .to_string(),
        ));
        for (delay, delta) in deltas {
            self.events.push((
                delay,
                "content_block_delta",
                json!({"type": "content_block_delta", "index": index, "delta": delta}).to_string(),
            ));
        }
        self.events.push((
            10,
            "content_block_stop",
            json!({"type": "content_block_stop", "index": index}).to_string(),
        ));
    }

    fn thinking(&mut self, text: &str) {
        let mut deltas: Vec<(u64, Value)> = words(text, 3)
            .into_iter()
            .map(|chunk| (60, json!({"type": "thinking_delta", "thinking": chunk})))
            .collect();
        deltas.push((
            10,
            json!({"type": "signature_delta", "signature": format!("fake-signature-{}", self.index)}),
        ));
        self.block(
            json!({"type": "thinking", "thinking": "", "signature": ""}),
            deltas,
        );
    }

    fn text(&mut self, text: &str) {
        self.slow_text(text, 30);
    }

    fn slow_text(&mut self, text: &str, delay: u64) {
        let deltas = words(text, 3)
            .into_iter()
            .map(|chunk| (delay, json!({"type": "text_delta", "text": chunk})))
            .collect();
        self.block(json!({"type": "text", "text": ""}), deltas);
    }

    fn tool(&mut self, name: &str, input: &Value) {
        let id = format!("toolu_fake{}", self.index);
        let input = input.to_string();
        // Fragments of a few characters, like the API's.
        let chars: Vec<char> = input.chars().collect();
        let deltas = std::iter::once(String::new())
            .chain(chars.chunks(6).map(|chunk| chunk.iter().collect()))
            .map(|fragment| {
                (
                    40,
                    json!({"type": "input_json_delta", "partial_json": fragment}),
                )
            })
            .collect();
        self.block(
            json!({"type": "tool_use", "id": id, "name": name, "input": {}}),
            deltas,
        );
    }

    fn end(&mut self, stop_reason: &str) {
        self.events.push((
            10,
            "message_delta",
            json!({"type": "message_delta", "delta": {"stop_reason": stop_reason, "stop_sequence": null}, "usage": {"output_tokens": 42}})
                .to_string(),
        ));
        self.events.push((
            10,
            "message_stop",
            json!({"type": "message_stop"}).to_string(),
        ));
    }

    fn finish(mut self) -> Vec<(u64, &'static str, String)> {
        let start = json!({"type": "message_start", "message": {
            "id": "msg_fake", "type": "message", "role": "assistant", "content": [],
            "model": "claude-sonnet-5-5", "stop_reason": null, "stop_sequence": null,
            "usage": {"input_tokens": 10, "output_tokens": 1}
        }});
        self.events
            .insert(0, (300, "message_start", start.to_string()));
        self.events
            .insert(1, (0, "ping", json!({"type": "ping"}).to_string()));
        self.events
    }
}

/// `text` in chunks of about `count` words, keeping the whitespace.
fn words(text: &str, count: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut seen = 0;
    for piece in text.split_inclusive(' ') {
        current.push_str(piece);
        seen += 1;
        if seen == count {
            chunks.push(std::mem::take(&mut current));
            seen = 0;
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// The OpenAI mode: the Responses API's streaming events and error bodies.
mod responses {
    use std::collections::HashSet;

    use serde_json::{Value, json};
    use tokio::net::TcpStream;

    use super::{Recorded, expression_in, words, write_json, write_stream};

    /// An error response a marker in the message asks for.
    struct Failure {
        marker: &'static str,
        status: u16,
        message: &'static str,
        error_type: Option<&'static str>,
        code: Option<&'static str>,
        retry_after: Option<u64>,
    }

    /// Statuses, types and codes from the error codes guide.
    const FAILURES: [Failure; 5] = [
        Failure {
            marker: "[401]",
            status: 401,
            message: "Incorrect API key provided.",
            error_type: None,
            code: None,
            retry_after: None,
        },
        Failure {
            marker: "[402]",
            status: 429,
            message: "You exceeded your current quota. Add credits to continue using the API.",
            error_type: Some("insufficient_quota"),
            code: Some("insufficient_quota"),
            retry_after: None,
        },
        Failure {
            marker: "[429]",
            status: 429,
            message: "Rate limit reached for requests.",
            error_type: Some("rate_limit_error"),
            code: Some("rate_limit_exceeded"),
            retry_after: Some(7),
        },
        Failure {
            marker: "[500]",
            status: 500,
            message: "The server had an error while processing your request.",
            error_type: Some("server_error"),
            code: None,
            retry_after: None,
        },
        Failure {
            marker: "[503]",
            status: 503,
            message: "The model is temporarily overloaded.",
            error_type: Some("service_unavailable_error"),
            code: Some("server_is_overloaded"),
            retry_after: None,
        },
    ];

    pub async fn serve(socket: &mut TcpStream, request: &Recorded) -> std::io::Result<()> {
        let key = request
            .authorization
            .strip_prefix("Bearer ")
            .unwrap_or_default();
        if !key.starts_with("sk-") || key.starts_with("sk-ant-") {
            return error(socket, 401, "Incorrect API key provided.", None, None, None).await;
        }
        if let Err(problem) = check(&request.body) {
            return error(
                socket,
                400,
                &problem,
                Some("invalid_request_error"),
                None,
                None,
            )
            .await;
        }
        let input = request.body["input"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let last = input.last().cloned().unwrap_or(Value::Null);
        let text = if last["type"] == "function_call_output" {
            String::new()
        } else {
            message_text(&last)
        };

        if last["type"] != "function_call_output" {
            if text.contains("[echokey]") {
                let message = format!(
                    "Incorrect API key provided: {key}. You can find your API key at \
                     https://platform.openai.com/api-keys."
                );
                return error(socket, 401, &message, None, None, None).await;
            }
            for failure in FAILURES {
                if text.contains(failure.marker) {
                    return error(
                        socket,
                        failure.status,
                        failure.message,
                        failure.error_type,
                        failure.code,
                        failure.retry_after,
                    )
                    .await;
                }
            }
        }
        write_stream(socket, script(&last, &text)).await
    }

    /// Writes an error body: `{"error": {"message", "type", "param", "code"}}`.
    async fn error(
        socket: &mut TcpStream,
        status: u16,
        message: &str,
        error_type: Option<&str>,
        code: Option<&str>,
        retry_after: Option<u64>,
    ) -> std::io::Result<()> {
        let body = json!({
            "error": { "message": message, "type": error_type, "param": null, "code": code }
        });
        write_json(socket, status, &body, retry_after).await
    }

    /// The text of a message item: string content, or its text parts.
    fn message_text(item: &Value) -> String {
        match &item["content"] {
            Value::String(text) => text.clone(),
            Value::Array(parts) => parts
                .iter()
                .filter_map(|part| part["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        }
    }

    /// Checks what the demo must send: a streaming, unstored request whose input
    /// items are well formed, with every function call answered by its output.
    fn check(body: &Value) -> Result<(), String> {
        if !body["model"].is_string() || !body["instructions"].is_string() {
            return Err("model and instructions are required".to_owned());
        }
        if body["stream"] != true || body["store"] != false {
            return Err("this fake expects stream: true and store: false".to_owned());
        }
        let tools = body["tools"].as_array().ok_or("tools must be an array")?;
        for tool in tools {
            if tool["type"] != "function"
                || !tool["name"].is_string()
                || !tool["parameters"].is_object()
            {
                return Err(format!("invalid function tool: {tool}"));
            }
        }
        let input = body["input"].as_array().ok_or("input must be an array")?;
        let mut calls: Vec<&str> = Vec::new();
        let mut answered: HashSet<&str> = HashSet::new();
        for item in input {
            let string = |field: &str| item[field].as_str();
            match item["type"].as_str() {
                Some("message") | None => {
                    let role_ok = matches!(string("role"), Some("user" | "assistant"));
                    let content_ok = match &item["content"] {
                        Value::String(text) => !text.is_empty(),
                        Value::Array(parts) => !parts.is_empty(),
                        _ => false,
                    };
                    if !role_ok || !content_ok {
                        return Err(format!("invalid message item: {item}"));
                    }
                }
                Some("reasoning") => {
                    if string("id").is_none()
                        || !item["summary"].is_array()
                        || string("encrypted_content").is_none()
                    {
                        return Err(format!("invalid reasoning item: {item}"));
                    }
                }
                Some("function_call") => {
                    let (Some(call_id), Some(_), Some(_)) =
                        (string("call_id"), string("name"), string("arguments"))
                    else {
                        return Err(format!("invalid function_call item: {item}"));
                    };
                    calls.push(call_id);
                }
                Some("function_call_output") => {
                    let call_id = string("call_id").unwrap_or_default();
                    if !calls.contains(&call_id) || string("output").is_none() {
                        return Err(format!(
                            "No tool call found for function call output {item}"
                        ));
                    }
                    answered.insert(call_id);
                }
                Some(other) => return Err(format!("unexpected input item type {other}")),
            }
        }
        if let Some(call_id) = calls.iter().find(|call_id| !answered.contains(*call_id)) {
            return Err(format!("No tool output found for function call {call_id}."));
        }
        match input.last() {
            Some(item)
                if item["type"] == "function_call_output"
                    || (item["role"] == "user" && item["type"] != "function_call") =>
            {
                Ok(())
            }
            _ => Err("the input must end with a user message or a tool output".to_owned()),
        }
    }

    fn script(last: &Value, text: &str) -> Vec<(u64, &'static str, String)> {
        let mut stream = Stream::default();
        if last["type"] == "function_call_output" {
            let result = last["output"].as_str().unwrap_or_default();
            stream.reasoning("The tool answered, so I can reply with its result.");
            let summary = if result.lines().count() == 1 {
                format!("`{result}`")
            } else {
                format!("{} lines", result.lines().count())
            };
            stream.message(
                &format!("The tool returned {summary}. That answers the question."),
                30,
            );
            stream.completed();
        } else if text.contains("[midstream]") {
            stream.partial_message("This reply starts, and then");
            stream.event(
                "error",
                json!({"code": "server_is_overloaded", "message": "The model is overloaded.",
                    "param": null}),
            );
        } else if text.contains("[slow]") {
            stream.message(
                &"This reply streams slowly so there is time to stop it. ".repeat(4),
                150,
            );
            stream.completed();
        } else if text.contains("[refusal]") {
            stream.refusal("I can't help with that.");
            stream.completed();
        } else if text.contains("[maxtokens]") {
            stream.message("A very long answer that runs out of room", 30);
            stream.event(
                "response.incomplete",
                json!({"response": stream.response("incomplete", json!({"reason": "max_output_tokens"}))}),
            );
        } else if text.contains("[nothink]") {
            // A model or organization without summaries: the reasoning item has none.
            stream.reasoning("");
            stream.message("A plain answer, with no reasoning summary to show.", 30);
            stream.completed();
        } else if text.contains("[text]") {
            stream.reasoning("A plain answer with some code will do.");
            stream.message(
                "Here is a **Markdown** answer from the fake API.\n\n```rust\nfn main() {\n    \
                 println!(\"hello\");\n}\n```\n\n- one\n- two",
                30,
            );
            stream.completed();
        } else if text.contains("[badargs]") {
            stream.reasoning("I'll use the calculate tool.");
            stream.message("Let me work that out.", 30);
            stream.function_call("calculate", "{\"expression\": \"1+");
            stream.completed();
        } else if text.to_lowercase().contains("component") {
            stream.reasoning("The question is about components; I'll search the catalog.");
            stream.message("Let me search the component catalog.", 30);
            stream.function_call(
                "search_components",
                &json!({ "query": "reasoning" }).to_string(),
            );
            stream.completed();
        } else {
            let expression = expression_in(text).unwrap_or_else(|| "6 * 7".to_owned());
            stream.reasoning(&format!(
                "The user wants a number. I'll use the calculate tool on `{expression}`."
            ));
            stream.message("Let me work that out.", 30);
            stream.function_call(
                "calculate",
                &json!({ "expression": expression }).to_string(),
            );
            stream.completed();
        }
        stream.finish()
    }

    /// Builds a stream's events with the shapes in the streaming events reference.
    #[derive(Default)]
    struct Stream {
        events: Vec<(u64, &'static str, String)>,
        sequence: u64,
        /// The finished output items, for `response.completed`.
        output: Vec<Value>,
    }

    impl Stream {
        fn event(&mut self, name: &'static str, fields: Value) {
            self.timed(20, name, fields);
        }

        fn timed(&mut self, delay: u64, name: &'static str, mut fields: Value) {
            fields["type"] = json!(name);
            fields["sequence_number"] = json!(self.sequence);
            self.sequence += 1;
            self.events.push((delay, name, fields.to_string()));
        }

        fn response(&self, status: &str, incomplete_details: Value) -> Value {
            json!({
                "id": "resp_fake", "object": "response", "created_at": 1_764_000_000,
                "status": status, "error": null, "incomplete_details": incomplete_details,
                "model": "gpt-6.2-sol", "output": self.output, "store": false,
            })
        }

        fn added(&mut self, item: &Value) {
            let output_index = self.output.len();
            self.event(
                "response.output_item.added",
                json!({"output_index": output_index, "item": item}),
            );
        }

        fn done(&mut self, item: Value) {
            let output_index = self.output.len();
            self.event(
                "response.output_item.done",
                json!({"output_index": output_index, "item": item}),
            );
            self.output.push(item);
        }

        /// A reasoning item with one summary part, or none when `summary` is empty.
        fn reasoning(&mut self, summary: &str) {
            let output_index = self.output.len();
            let id = format!("rs_fake{output_index}");
            self.added(&json!({"id": id, "type": "reasoning", "summary": []}));
            let mut summaries = Vec::new();
            if !summary.is_empty() {
                let at = json!({"item_id": id, "output_index": output_index, "summary_index": 0});
                let with = |extra: Value| {
                    let mut fields = at.clone();
                    fields
                        .as_object_mut()
                        .unwrap()
                        .extend(extra.as_object().unwrap().clone());
                    fields
                };
                self.event(
                    "response.reasoning_summary_part.added",
                    with(json!({"part": {"type": "summary_text", "text": ""}})),
                );
                for chunk in words(summary, 3) {
                    self.timed(
                        60,
                        "response.reasoning_summary_text.delta",
                        with(json!({"delta": chunk})),
                    );
                }
                self.event(
                    "response.reasoning_summary_text.done",
                    with(json!({"text": summary})),
                );
                self.event(
                    "response.reasoning_summary_part.done",
                    with(json!({"part": {"type": "summary_text", "text": summary}})),
                );
                summaries.push(json!({"type": "summary_text", "text": summary}));
            }
            self.done(json!({
                "id": id, "type": "reasoning", "summary": summaries,
                "encrypted_content": format!("fake-encrypted-{output_index}"),
            }));
        }

        /// The events of a message up to its text deltas.
        fn message_start(&mut self, text: &str, delay: u64) -> (String, usize) {
            let output_index = self.output.len();
            let id = format!("msg_fake{output_index}");
            self.added(
                &json!({"id": id, "status": "in_progress", "type": "message",
                "role": "assistant", "content": []}),
            );
            self.event(
                "response.content_part.added",
                json!({"item_id": id, "output_index": output_index, "content_index": 0,
                    "part": {"type": "output_text", "text": "", "annotations": [], "logprobs": []}}),
            );
            for chunk in words(text, 3) {
                self.timed(
                    delay,
                    "response.output_text.delta",
                    json!({"item_id": id, "output_index": output_index, "content_index": 0,
                        "delta": chunk, "logprobs": []}),
                );
            }
            (id, output_index)
        }

        fn message(&mut self, text: &str, delay: u64) {
            let (id, output_index) = self.message_start(text, delay);
            let part =
                json!({"type": "output_text", "text": text, "annotations": [], "logprobs": []});
            self.event(
                "response.output_text.done",
                json!({"item_id": id, "output_index": output_index, "content_index": 0,
                    "text": text, "logprobs": []}),
            );
            self.event(
                "response.content_part.done",
                json!({"item_id": id, "output_index": output_index, "content_index": 0,
                    "part": part}),
            );
            self.done(json!({"id": id, "status": "completed", "type": "message",
                "role": "assistant", "content": [part]}));
        }

        /// A message that stops after its text, as when the stream fails.
        fn partial_message(&mut self, text: &str) {
            self.message_start(text, 30);
        }

        fn refusal(&mut self, refusal: &str) {
            let output_index = self.output.len();
            let id = format!("msg_fake{output_index}");
            let at = json!({"item_id": id, "output_index": output_index, "content_index": 0});
            let with = |extra: Value| {
                let mut fields = at.clone();
                fields
                    .as_object_mut()
                    .unwrap()
                    .extend(extra.as_object().unwrap().clone());
                fields
            };
            self.added(
                &json!({"id": id, "status": "in_progress", "type": "message",
                "role": "assistant", "content": []}),
            );
            self.event(
                "response.content_part.added",
                with(json!({"part": {"type": "refusal", "refusal": ""}})),
            );
            self.event("response.refusal.delta", with(json!({"delta": refusal})));
            self.event("response.refusal.done", with(json!({"refusal": refusal})));
            let part = json!({"type": "refusal", "refusal": refusal});
            self.event("response.content_part.done", with(json!({"part": part})));
            self.done(json!({"id": id, "status": "completed", "type": "message",
                "role": "assistant", "content": [part]}));
        }

        fn function_call(&mut self, name: &str, arguments: &str) {
            let output_index = self.output.len();
            let id = format!("fc_fake{output_index}");
            let call_id = format!("call_fake{output_index}");
            self.added(
                &json!({"id": id, "type": "function_call", "status": "in_progress",
                "call_id": call_id, "name": name, "arguments": ""}),
            );
            // Fragments of a few characters, like the API's.
            let chars: Vec<char> = arguments.chars().collect();
            for chunk in chars.chunks(6) {
                self.timed(
                    40,
                    "response.function_call_arguments.delta",
                    json!({"item_id": id, "output_index": output_index,
                        "delta": chunk.iter().collect::<String>()}),
                );
            }
            self.event(
                "response.function_call_arguments.done",
                json!({"item_id": id, "output_index": output_index, "arguments": arguments}),
            );
            self.done(
                json!({"id": id, "type": "function_call", "status": "completed",
                "call_id": call_id, "name": name, "arguments": arguments}),
            );
        }

        fn completed(&mut self) {
            let response = self.response("completed", Value::Null);
            self.event("response.completed", json!({ "response": response }));
        }

        fn finish(self) -> Vec<(u64, &'static str, String)> {
            let mut events = Vec::with_capacity(self.events.len() + 2);
            let mut start = Self::default();
            let created = start.response("in_progress", Value::Null);
            start.timed(300, "response.created", json!({ "response": created }));
            start.timed(0, "response.in_progress", json!({ "response": created }));
            events.extend(start.events);
            // Renumber so the sequence counts up from the created event.
            for (number, (delay, name, data)) in self.events.into_iter().enumerate() {
                let mut fields: Value = serde_json::from_str(&data).unwrap();
                fields["sequence_number"] = json!(number + 2);
                events.push((delay, name, fields.to_string()));
            }
            events
        }
    }
}
