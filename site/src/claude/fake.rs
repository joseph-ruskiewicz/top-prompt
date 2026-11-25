//! A stand-in for the Messages API, for tests and for trying the demo locally without
//! a key. Compiled only in tests and debug builds.
//!
//! It speaks just enough HTTP/1.1 for one request per connection and answers with the
//! event and error shapes from the Claude API docs. The reply depends on the last
//! user message:
//!
//! - after tool results: thinking, then text quoting the first result;
//! - `[401]`, `[402]`, `[429]`, `[500]`, `[529]`: that error response;
//! - `[midstream]`: some text, then an `overloaded_error` event;
//! - `[slow]`: text that streams for several seconds, to try stopping;
//! - `[refusal]` and `[maxtokens]`: text that ends with that stop reason;
//! - `[text]`: thinking and a Markdown answer, with no tool call;
//! - anything else: thinking, a sentence, and a tool call: `search_components` when
//!   the message mentions a component, `calculate` otherwise.

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

use super::api::API_VERSION;

/// A running fake API.
pub struct FakeApi {
    url: String,
    #[cfg_attr(not(test), allow(dead_code))]
    requests: Arc<Mutex<Vec<Recorded>>>,
}

/// A request the fake received. Deliberately not `Debug`: it holds the key.
#[derive(Clone)]
pub struct Recorded {
    pub api_key: String,
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

    if request.path != "/v1/messages" {
        return write_error(&mut socket, 404, "not_found_error", "Not found", None).await;
    }
    if !request.api_key.starts_with("sk-ant-") {
        return write_error(
            &mut socket,
            401,
            "authentication_error",
            "invalid x-api-key",
            None,
        )
        .await;
    }
    if request.version != API_VERSION {
        return write_error(
            &mut socket,
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
        } => write_error(&mut socket, status, error_type, message, retry_after).await,
        Script::Stream(events) => {
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
    }
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
    let mut version = String::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match name.trim().to_ascii_lowercase().as_str() {
            "content-length" => length = value.parse().unwrap_or(0),
            "x-api-key" => api_key = value.to_owned(),
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
    })
    .to_string();
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
