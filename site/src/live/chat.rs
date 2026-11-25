//! Each visitor's key and conversation, and the task that streams a reply into it.
//!
//! [`Demo`] lives in router context and maps session token hashes to [`Visitor`]s.
//! A visitor holds the key in memory only, beside the conversation; nothing here is
//! written anywhere else. Idle visitors are dropped, key and all, by a sweeper task.

use std::{
    collections::{HashMap, VecDeque},
    net::IpAddr,
    sync::{Arc, Mutex, Once, Weak},
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use tokio::{sync::broadcast, task::AbortHandle, time::sleep};
use topcoat::session::TokenHash;

use super::{
    api::{self, Accumulator, ApiError, Update},
    key::{ApiKey, KeyError},
    provider::{Model, Provider},
    tools,
};
use crate::components::{prompt_input::PromptStatus, tool_call::ToolStatus};

/// The longest message accepted, in bytes.
pub const MAX_MESSAGE_LEN: usize = 4000;
/// The most user messages in one conversation.
pub const MAX_TURNS: usize = 20;
/// The most tool-use round trips for one reply.
pub const MAX_TOOL_ROUNDS: usize = 5;
/// API requests allowed per visitor in [`WINDOW`]. Each tool round trip is one.
pub const VISITOR_REQUESTS: usize = 20;
/// API requests allowed per client IP address in [`WINDOW`], across its visitors.
pub const IP_REQUESTS: usize = 40;
pub const WINDOW: Duration = Duration::from_secs(10 * 60);
/// A visitor untouched for this long is dropped, key included.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
/// The most visitors held at once.
pub const MAX_VISITORS: usize = 500;

/// Every visitor's state, shared through router context.
#[derive(Clone)]
pub struct Demo {
    inner: Arc<DemoInner>,
}

struct DemoInner {
    client: api::Client,
    visitors: Mutex<HashMap<TokenHash, Arc<Visitor>>>,
    ips: Mutex<HashMap<IpAddr, Window>>,
    sweeper: Once,
}

impl Demo {
    pub fn new(endpoint: api::Endpoint) -> Self {
        Self {
            inner: Arc::new(DemoInner {
                client: api::Client::new(endpoint),
                visitors: Mutex::default(),
                ips: Mutex::default(),
                sweeper: Once::new(),
            }),
        }
    }

    /// The visitor for a session, created on first use. `None` when the demo holds
    /// [`MAX_VISITORS`] that are all active.
    pub fn visitor(&self, session: &TokenHash) -> Option<Arc<Visitor>> {
        self.start_sweeper();
        let mut visitors = self.inner.visitors.lock().unwrap();
        if let Some(visitor) = visitors.get(session) {
            visitor.touch();
            return Some(visitor.clone());
        }
        if visitors.len() >= MAX_VISITORS {
            let evicted = evict_idle(&mut visitors, Duration::ZERO, 1);
            if evicted.is_empty() {
                return None;
            }
        }
        let visitor = Arc::new(Visitor::new());
        visitors.insert(session.clone(), visitor.clone());
        Some(visitor)
    }

    /// Drops visitors idle for [`IDLE_TIMEOUT`] once a minute. The task holds only a
    /// weak reference, so it ends with the demo.
    fn start_sweeper(&self) {
        self.inner.sweeper.call_once(|| {
            let demo: Weak<DemoInner> = Arc::downgrade(&self.inner);
            tokio::spawn(async move {
                loop {
                    sleep(Duration::from_secs(60)).await;
                    let Some(demo) = demo.upgrade() else {
                        return;
                    };
                    demo.sweep(IDLE_TIMEOUT);
                }
            });
        });
    }

    /// Records one API request for `visitor` and `ip`, or says which limit stops it.
    fn take_request(&self, visitor: &Visitor, ip: Option<IpAddr>) -> Result<(), String> {
        let now = Instant::now();
        if let Some(ip) = ip {
            let mut ips = self.inner.ips.lock().unwrap();
            ips.retain(|_, window| !window.is_empty(now));
            let window = ips.entry(ip).or_default();
            if !window.allows(now, IP_REQUESTS) {
                return Err(format!(
                    "This network has made {IP_REQUESTS} requests in the last 10 minutes, the \
                     demo's limit. Try again later."
                ));
            }
            window.record(now);
        }
        let mut state = visitor.state.lock().unwrap();
        if !state.requests.allows(now, VISITOR_REQUESTS) {
            drop(state);
            if let Some(ip) = ip
                && let Some(window) = self.inner.ips.lock().unwrap().get_mut(&ip)
            {
                window.forget_last();
            }
            return Err(format!(
                "You have made {VISITOR_REQUESTS} requests in the last 10 minutes, the demo's \
                 limit. Try again in a few minutes."
            ));
        }
        state.requests.record(now);
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn sweep_now(&self, idle: Duration) {
        self.inner.sweep(idle);
    }

    #[cfg(test)]
    pub(super) fn visitor_count(&self) -> usize {
        self.inner.visitors.lock().unwrap().len()
    }
}

impl DemoInner {
    fn sweep(&self, idle: Duration) {
        let evicted = evict_idle(&mut self.visitors.lock().unwrap(), idle, usize::MAX);
        // Wake the pages watching them, which then render a fresh visitor.
        for visitor in evicted {
            visitor.notify();
        }
    }
}

/// Removes up to `limit` visitors idle for at least `idle` and not generating, the
/// least recently used first.
fn evict_idle(
    visitors: &mut HashMap<TokenHash, Arc<Visitor>>,
    idle: Duration,
    limit: usize,
) -> Vec<Arc<Visitor>> {
    let now = Instant::now();
    let mut idle: Vec<(Instant, TokenHash)> = visitors
        .iter()
        .filter_map(|(hash, visitor)| {
            let state = visitor.state.lock().unwrap();
            (state.generation.is_none() && now.duration_since(state.last_seen) >= idle)
                .then(|| (state.last_seen, hash.clone()))
        })
        .collect();
    idle.sort_by_key(|(seen, _)| *seen);
    idle.into_iter()
        .take(limit)
        .filter_map(|(_, hash)| visitors.remove(&hash))
        .collect()
}

/// Request times in a sliding window.
#[derive(Default)]
struct Window(VecDeque<Instant>);

impl Window {
    fn prune(&mut self, now: Instant) {
        while self
            .0
            .front()
            .is_some_and(|at| now.duration_since(*at) >= WINDOW)
        {
            self.0.pop_front();
        }
    }

    fn allows(&mut self, now: Instant, limit: usize) -> bool {
        self.prune(now);
        self.0.len() < limit
    }

    fn record(&mut self, now: Instant) {
        self.0.push_back(now);
    }

    fn forget_last(&mut self) {
        self.0.pop_back();
    }

    fn is_empty(&mut self, now: Instant) -> bool {
        self.prune(now);
        self.0.is_empty()
    }
}

/// One visitor: a browser session.
pub struct Visitor {
    state: Mutex<State>,
    changed: broadcast::Sender<()>,
}

struct State {
    /// Held in memory only. Never cloned out except into the generation task.
    key: Option<Arc<ApiKey>>,
    /// Whose API the key is for. The model always belongs to it.
    provider: Provider,
    model: Model,
    turns: Vec<Turn>,
    next_id: u64,
    /// The task streaming a reply, if one is running.
    generation: Option<Generation>,
    next_generation: u64,
    requests: Window,
    last_seen: Instant,
}

/// A running reply task. Its updates apply only while it is the current one, so a
/// task stopped just as it finished cannot add to a reply after the stop.
struct Generation {
    id: u64,
    handle: AbortHandle,
}

/// A user message and the replies generated for it.
struct Turn {
    id: u64,
    user: String,
    replies: Vec<Reply>,
    current: usize,
}

/// One reply: what the page shows, and the messages it adds to the API history.
struct Reply {
    view: ReplyView,
    /// Assistant messages and tool-result user messages, in order. Append-only, so
    /// thinking blocks go back to the API exactly as they came.
    api: Vec<Value>,
    /// The first part of the API request being streamed, so a stop knows which
    /// parts are not in `api` yet.
    round_start: usize,
}

/// A visitor as the page renders it. It never holds the key.
#[derive(Clone, Default)]
pub struct Snapshot {
    pub has_key: bool,
    #[allow(dead_code, reason = "the page shows it once it has a provider picker")]
    pub provider: Provider,
    pub model: Model,
    pub turns: Vec<TurnView>,
    pub status: PromptStatus,
    /// No more messages fit in this conversation.
    pub full: bool,
}

#[derive(Clone)]
pub struct TurnView {
    pub id: u64,
    pub user: String,
    pub replies: Vec<ReplyView>,
    pub current: usize,
}

#[derive(Clone, Default)]
pub struct ReplyView {
    pub state: ReplyState,
    pub model: Model,
    pub parts: Vec<Part>,
    pub notice: Option<Notice>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum ReplyState {
    /// Sent; nothing has arrived yet.
    #[default]
    Waiting,
    Streaming,
    Done,
    Stopped,
    Failed,
}

/// A note under a reply: an error, or why it ended early.
#[derive(Clone, Debug)]
pub struct Notice {
    pub text: String,
    pub error: bool,
    /// The key was rejected, so the notice offers to replace it.
    pub key_problem: bool,
}

impl Notice {
    fn warning(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            error: false,
            key_problem: false,
        }
    }
}

/// A piece of a reply, in the order the model produced it.
#[derive(Clone, Debug)]
pub enum Part {
    Thinking {
        text: String,
        streaming: bool,
        started: Instant,
        secs: Option<u32>,
    },
    Tool {
        name: String,
        status: ToolStatus,
        /// The input as it streams, then pretty-printed.
        input: String,
        output: Option<String>,
        error: bool,
    },
    Text(String),
}

impl ReplyView {
    pub fn finished(&self) -> bool {
        matches!(
            self.state,
            ReplyState::Done | ReplyState::Stopped | ReplyState::Failed
        )
    }

    /// The reply's text, for the copy button.
    pub fn text(&self) -> String {
        let texts: Vec<&str> = self
            .parts
            .iter()
            .filter_map(|part| match part {
                Part::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        texts.join("\n\n")
    }
}

impl Visitor {
    fn new() -> Self {
        Self {
            state: Mutex::new(State {
                key: None,
                provider: Provider::default(),
                model: Provider::default().default_model(),
                turns: Vec::new(),
                next_id: 0,
                generation: None,
                next_generation: 0,
                requests: Window::default(),
                last_seen: Instant::now(),
            }),
            changed: broadcast::channel(64).0,
        }
    }

    /// Subscribe before reading a snapshot, so a change between the two is not missed.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.changed.subscribe()
    }

    fn notify(&self) {
        let _ = self.changed.send(());
    }

    fn touch(&self) {
        self.state.lock().unwrap().last_seen = Instant::now();
    }

    pub fn snapshot(&self) -> Snapshot {
        let state = self.state.lock().unwrap();
        let status = match state.turns.last() {
            Some(turn) if state.generation.is_some() => {
                if turn.replies[turn.current].view.state == ReplyState::Waiting {
                    PromptStatus::Submitted
                } else {
                    PromptStatus::Streaming
                }
            }
            _ => PromptStatus::Ready,
        };
        Snapshot {
            has_key: state.key.is_some(),
            provider: state.provider,
            model: state.model,
            turns: state
                .turns
                .iter()
                .map(|turn| TurnView {
                    id: turn.id,
                    user: turn.user.clone(),
                    replies: turn
                        .replies
                        .iter()
                        .map(|reply| reply.view.clone())
                        .collect(),
                    current: turn.current,
                })
                .collect(),
            status,
            full: state.turns.len() >= MAX_TURNS,
        }
    }

    /// Stores a key the visitor entered, checked against the current provider. `Err`
    /// is a message for the page.
    pub fn set_key(&self, raw: &str) -> Result<(), &'static str> {
        let mut state = self.state.lock().unwrap();
        let key = ApiKey::parse(state.provider, raw).map_err(KeyError::message)?;
        state.key = Some(Arc::new(key));
        state.last_seen = Instant::now();
        drop(state);
        self.notify();
        Ok(())
    }

    /// Forgets the key and the conversation: thinking blocks only work with the
    /// account that produced them, and the conversation was paid for by that key.
    pub fn clear_key(&self) {
        let mut state = self.state.lock().unwrap();
        if let Some(generation) = state.generation.take() {
            generation.handle.abort();
        }
        state.key = None;
        state.turns.clear();
        drop(state);
        self.notify();
    }

    /// Switches provider. The key and the conversation belong to the old one, so
    /// they go, and any reply being generated stops. Returns whether it changed.
    #[allow(dead_code, reason = "the page calls it once it has a provider picker")]
    pub fn set_provider(&self, provider: Provider) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.provider == provider {
            return false;
        }
        // Taking the generation makes any late update from its task a no-op.
        if let Some(generation) = state.generation.take() {
            generation.handle.abort();
        }
        state.provider = provider;
        state.model = provider.default_model();
        state.key = None;
        state.turns.clear();
        drop(state);
        self.notify();
        true
    }

    /// Picks the model for the next replies. Only the current provider's models,
    /// and not while a reply is being generated.
    pub fn set_model(&self, model: Model) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.generation.is_some() || model.provider() != state.provider {
            return false;
        }
        state.model = model;
        drop(state);
        self.notify();
        true
    }

    /// Adds a user message and starts a reply. Returns `false` if there is no key,
    /// a reply is in progress, the message is empty or too long, or the
    /// conversation is full.
    pub fn send(self: &Arc<Self>, demo: &Demo, text: &str, ip: Option<IpAddr>) -> bool {
        let text = text.trim();
        if text.is_empty() || text.len() > MAX_MESSAGE_LEN {
            return false;
        }
        let mut state = self.state.lock().unwrap();
        if state.generation.is_some() || state.turns.len() >= MAX_TURNS {
            return false;
        }
        let Some(key) = state.key.clone() else {
            return false;
        };
        let id = state.next_id;
        state.next_id += 1;
        let model = state.model;
        state.turns.push(Turn {
            id,
            user: text.to_owned(),
            replies: vec![Reply::new(model)],
            current: 0,
        });
        let messages = history(&state.turns);
        state.start(self, demo, key, model, ip, messages);
        drop(state);
        self.notify();
        true
    }

    /// Generates another reply to the last message, as a new branch.
    pub fn regenerate(self: &Arc<Self>, demo: &Demo, ip: Option<IpAddr>) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.generation.is_some() {
            return false;
        }
        let Some(key) = state.key.clone() else {
            return false;
        };
        let model = state.model;
        let Some(turn) = state.turns.last_mut() else {
            return false;
        };
        turn.replies.push(Reply::new(model));
        turn.current = turn.replies.len() - 1;
        let messages = history(&state.turns);
        state.start(self, demo, key, model, ip, messages);
        drop(state);
        self.notify();
        true
    }

    pub fn select_branch(&self, turn_id: u64, branch: usize) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.generation.is_some() {
            return false;
        }
        let Some(turn) = state.turns.iter_mut().find(|turn| turn.id == turn_id) else {
            return false;
        };
        if branch >= turn.replies.len() {
            return false;
        }
        turn.current = branch;
        drop(state);
        self.notify();
        true
    }

    /// Stops the reply in progress, keeping what has arrived.
    pub fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        let Some(generation) = state.generation.take() else {
            return;
        };
        generation.handle.abort();
        if let Some(reply) = state.current_reply() {
            reply.interrupt(ReplyState::Stopped, None);
        }
        drop(state);
        self.notify();
    }

    /// Clears the conversation, keeping the key.
    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        if let Some(generation) = state.generation.take() {
            generation.handle.abort();
        }
        state.turns.clear();
        drop(state);
        self.notify();
    }

    #[cfg(test)]
    pub(super) fn api_history(&self) -> Vec<Value> {
        history(&self.state.lock().unwrap().turns)
    }

    #[cfg(test)]
    pub(super) fn age(&self, by: Duration) {
        let mut state = self.state.lock().unwrap();
        state.last_seen -= by;
    }
}

impl State {
    /// Spawns the task that streams the reply to the last turn.
    fn start(
        &mut self,
        visitor: &Arc<Visitor>,
        demo: &Demo,
        key: Arc<ApiKey>,
        model: Model,
        ip: Option<IpAddr>,
        messages: Vec<Value>,
    ) {
        let id = self.next_generation;
        self.next_generation += 1;
        let job = Job {
            visitor: visitor.clone(),
            id,
        };
        let demo = demo.clone();
        // The caller holds the state lock until the handle is stored, so the task's
        // first update waits for it.
        let handle = tokio::spawn(async move {
            generate(&demo, &job, &key, model, ip, messages).await;
        })
        .abort_handle();
        self.generation = Some(Generation { id, handle });
        self.last_seen = Instant::now();
    }

    /// The reply being generated: the shown branch of the last turn.
    fn current_reply(&mut self) -> Option<&mut Reply> {
        let turn = self.turns.last_mut()?;
        turn.replies.get_mut(turn.current)
    }
}

impl Reply {
    fn new(model: Model) -> Self {
        Self {
            view: ReplyView {
                model,
                ..ReplyView::default()
            },
            api: Vec::new(),
            round_start: 0,
        }
    }

    /// Ends the reply early. The parts of the request in flight are not complete
    /// messages, so their text alone goes into the history, as an assistant message;
    /// unfinished thinking and tool calls are dropped from it and shown as cancelled.
    fn interrupt(&mut self, state: ReplyState, notice: Option<Notice>) {
        let text: Vec<&str> = self.view.parts[self.round_start..]
            .iter()
            .filter_map(|part| match part {
                Part::Text(text) if !text.trim().is_empty() => Some(text.as_str()),
                _ => None,
            })
            .collect();
        if !text.is_empty() {
            self.api.push(json!({
                "role": "assistant",
                "content": [{ "type": "text", "text": text.join("\n\n") }],
            }));
        }
        for part in &mut self.view.parts[self.round_start..] {
            match part {
                Part::Thinking {
                    streaming,
                    secs,
                    started,
                    ..
                } if *streaming => {
                    *streaming = false;
                    *secs = Some(elapsed_secs(*started));
                }
                Part::Tool {
                    status,
                    output,
                    error,
                    ..
                } if matches!(status, ToolStatus::Pending | ToolStatus::Running) => {
                    *status = ToolStatus::Error;
                    *output = Some("Cancelled.".to_owned());
                    *error = true;
                }
                _ => {}
            }
        }
        self.round_start = self.view.parts.len();
        self.view.state = state;
        self.view.notice = notice;
    }
}

/// A reply task's handle on its visitor.
struct Job {
    visitor: Arc<Visitor>,
    id: u64,
}

impl Job {
    /// Applies `change` to the reply being generated, then notifies. Does nothing
    /// once this task is no longer the current one.
    fn update(&self, change: impl FnOnce(&mut Reply)) {
        let mut state = self.visitor.state.lock().unwrap();
        if !state
            .generation
            .as_ref()
            .is_some_and(|run| run.id == self.id)
        {
            return;
        }
        if let Some(reply) = state.current_reply() {
            change(reply);
        }
        drop(state);
        self.visitor.notify();
    }

    /// Ends the task, applying `change` to the reply first.
    fn finish(&self, change: impl FnOnce(&mut Reply)) {
        let mut state = self.visitor.state.lock().unwrap();
        if !state
            .generation
            .as_ref()
            .is_some_and(|run| run.id == self.id)
        {
            return;
        }
        if let Some(reply) = state.current_reply() {
            change(reply);
        }
        state.generation = None;
        state.last_seen = Instant::now();
        drop(state);
        self.visitor.notify();
    }
}

fn elapsed_secs(started: Instant) -> u32 {
    (started.elapsed().as_secs_f32().round() as u32).max(1)
}

/// The API messages for the conversation: each turn's user message, then the shown
/// reply's messages, except for the last turn, which is the one being answered.
///
/// A reply that ended before the model wrote anything, or after tool results, leaves
/// two user messages in a row; they are merged, results first, as the API requires.
fn history(turns: &[Turn]) -> Vec<Value> {
    let mut messages: Vec<Value> = Vec::new();
    for (index, turn) in turns.iter().enumerate() {
        push_merged(
            &mut messages,
            json!({ "role": "user", "content": [{ "type": "text", "text": turn.user }] }),
        );
        if index + 1 < turns.len() {
            for message in &turn.replies[turn.current].api {
                push_merged(&mut messages, message.clone());
            }
        }
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

/// Streams one reply, running tools until the model answers.
async fn generate(
    demo: &Demo,
    job: &Job,
    key: &ApiKey,
    model: Model,
    ip: Option<IpAddr>,
    mut messages: Vec<Value>,
) {
    for round in 0.. {
        if round == MAX_TOOL_ROUNDS {
            job.finish(|reply| {
                reply.view.state = ReplyState::Done;
                reply.view.notice = Some(Notice::warning(format!(
                    "Stopped after {MAX_TOOL_ROUNDS} rounds of tool calls, the demo's limit."
                )));
            });
            return;
        }
        if let Err(limit) = demo.take_request(&job.visitor, ip) {
            job.finish(|reply| {
                reply.interrupt(ReplyState::Failed, Some(Notice::warning(limit)));
            });
            return;
        }
        job.update(|reply| reply.round_start = reply.view.parts.len());

        let message = match stream_message(demo, job, key, model, &messages).await {
            Ok(message) => message,
            Err(error) => {
                let notice = Notice {
                    text: error.explain(model),
                    error: true,
                    key_problem: error.is_key_problem(),
                };
                job.finish(|reply| reply.interrupt(ReplyState::Failed, Some(notice)));
                return;
            }
        };

        match message.stop_reason.as_deref() {
            Some("tool_use") => {
                let results = run_tools(job, &message).await;
                let assistant = message.assistant_message();
                let results = json!({ "role": "user", "content": results });
                job.update(|reply| {
                    reply.api.push(assistant.clone());
                    reply.api.push(results.clone());
                    reply.round_start = reply.view.parts.len();
                });
                messages.push(assistant);
                messages.push(results);
            }
            Some("max_tokens") => {
                let notice = Notice::warning(format!(
                    "The reply reached the demo's cap of {} tokens.",
                    api::MAX_TOKENS
                ));
                job.finish(|reply| reply.interrupt(ReplyState::Done, Some(notice)));
                return;
            }
            Some("refusal") => {
                let notice = Notice::warning("Claude declined to continue with this request.");
                job.finish(|reply| reply.interrupt(ReplyState::Done, Some(notice)));
                return;
            }
            // end_turn, stop_sequence, and reasons added later.
            _ => {
                let assistant = message.assistant_message();
                job.finish(|reply| {
                    reply.api.push(assistant);
                    reply.round_start = reply.view.parts.len();
                    reply.view.state = ReplyState::Done;
                });
                return;
            }
        }
    }
}

/// Sends one request and streams its response into the reply's parts.
async fn stream_message(
    demo: &Demo,
    job: &Job,
    key: &ApiKey,
    model: Model,
    messages: &[Value],
) -> Result<Accumulator, ApiError> {
    let body = api::request_body(model, messages);
    let mut stream = demo.inner.client.stream(key, &body).await?;
    let mut message = Accumulator::default();
    // Content block index to part index, for this request.
    let mut parts: HashMap<usize, usize> = HashMap::new();
    while let Some(events) = stream.next().await? {
        let mut updates = Vec::new();
        for event in &events {
            if let Some(update) = message.apply(event, key)? {
                updates.push(update);
            }
        }
        if !updates.is_empty() {
            // Tool inputs are pretty-printed once complete.
            let inputs: HashMap<usize, String> = updates
                .iter()
                .filter_map(|update| match update {
                    Update::BlockStopped { index } => message
                        .content
                        .get(*index)
                        .filter(|block| block["type"] == "tool_use")
                        .map(|block| {
                            let input = match message.invalid_inputs.get(index) {
                                Some(raw) => raw.clone(),
                                None => serde_json::to_string_pretty(&block["input"])
                                    .unwrap_or_default(),
                            };
                            (*index, input)
                        }),
                    _ => None,
                })
                .collect();
            job.update(|reply| apply_updates(reply, &mut parts, updates, inputs));
        }
        if message.finished {
            return Ok(message);
        }
    }
    // The stream ended without message_stop.
    Err(ApiError::Transport)
}

fn apply_updates(
    reply: &mut Reply,
    parts: &mut HashMap<usize, usize>,
    updates: Vec<Update>,
    mut inputs: HashMap<usize, String>,
) {
    reply.view.state = ReplyState::Streaming;
    for update in updates {
        match update {
            Update::ThinkingStarted { index } => {
                parts.insert(index, reply.view.parts.len());
                reply.view.parts.push(Part::Thinking {
                    text: String::new(),
                    streaming: true,
                    started: Instant::now(),
                    secs: None,
                });
            }
            Update::TextStarted { index } => {
                parts.insert(index, reply.view.parts.len());
                reply.view.parts.push(Part::Text(String::new()));
            }
            Update::ToolStarted { index, name, .. } => {
                parts.insert(index, reply.view.parts.len());
                reply.view.parts.push(Part::Tool {
                    name,
                    status: ToolStatus::Pending,
                    input: String::new(),
                    output: None,
                    error: false,
                });
            }
            Update::Thinking { index, text: delta }
            | Update::Text { index, text: delta }
            | Update::ToolInput {
                index,
                partial: delta,
            } => {
                if let Some(part) = parts
                    .get(&index)
                    .and_then(|at| reply.view.parts.get_mut(*at))
                {
                    match part {
                        Part::Thinking { text, .. } | Part::Text(text) => text.push_str(&delta),
                        Part::Tool { input, .. } => input.push_str(&delta),
                    }
                }
            }
            Update::BlockStopped { index } => {
                if let Some(part) = parts
                    .get(&index)
                    .and_then(|at| reply.view.parts.get_mut(*at))
                {
                    match part {
                        Part::Thinking {
                            streaming,
                            started,
                            secs,
                            ..
                        } => {
                            *streaming = false;
                            *secs = Some(elapsed_secs(*started));
                        }
                        Part::Tool { input, .. } => {
                            if let Some(pretty) = inputs.remove(&index) {
                                *input = pretty;
                            }
                        }
                        Part::Text(_) => {}
                    }
                }
            }
        }
    }
}

/// Runs the tool calls of `message` and returns their `tool_result` blocks, all of
/// which go in one user message.
async fn run_tools(job: &Job, message: &Accumulator) -> Vec<Value> {
    let calls: Vec<(usize, String, String, Value)> = message
        .content
        .iter()
        .enumerate()
        .filter(|(_, block)| block["type"] == "tool_use")
        .map(|(index, block)| {
            (
                index,
                block["id"].as_str().unwrap_or_default().to_owned(),
                block["name"].as_str().unwrap_or_default().to_owned(),
                block["input"].clone(),
            )
        })
        .collect();

    job.update(|reply| {
        for part in &mut reply.view.parts[reply.round_start..] {
            if let Part::Tool { status, .. } = part {
                *status = ToolStatus::Running;
            }
        }
    });
    // The tools finish in microseconds; a short pause lets the page show the
    // running state before the result.
    sleep(Duration::from_millis(300)).await;

    let mut results = Vec::new();
    let mut outputs = Vec::new();
    for (index, id, name, input) in calls {
        let outcome = match message.invalid_inputs.get(&index) {
            Some(raw) => Err(format!(
                "INVALID_JSON: the input was not valid JSON, so the tool did not run. \
                 Received: {raw}"
            )),
            None => tools::run(&name, &input),
        };
        let (text, is_error) = match outcome {
            Ok(text) => (text, false),
            Err(text) => (text, true),
        };
        let mut result = json!({ "type": "tool_result", "tool_use_id": id, "content": text });
        if is_error {
            result["is_error"] = Value::Bool(true);
        }
        results.push(result);
        outputs.push((text, is_error));
    }

    job.update(|reply| {
        let tools = reply.view.parts[reply.round_start..]
            .iter_mut()
            .filter(|part| matches!(part, Part::Tool { .. }));
        for (part, (text, is_error)) in tools.zip(outputs) {
            if let Part::Tool {
                status,
                output,
                error,
                ..
            } = part
            {
                *status = if is_error {
                    ToolStatus::Error
                } else {
                    ToolStatus::Completed
                };
                *output = Some(text);
                *error = is_error;
            }
        }
    });
    results
}
