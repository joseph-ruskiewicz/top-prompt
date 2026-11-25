//! A working chat built from the components, answered by a fake model.
//!
//! The conversation lives in router context and is shared by every tab. Sending a
//! message starts a Tokio task that streams a canned response into it; the page's
//! live region only observes the conversation and renders it after every change, so
//! a reconnecting browser picks up where the stream is instead of restarting it.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tokio::{sync::broadcast, task::AbortHandle, time::sleep};
use topcoat::{
    Result,
    context::{Cx, context},
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, Signal, connected, procedure, shard, signal},
    view::{View, attributes, component, emit, live, view},
};

use crate::components::{
    branch_picker::{
        branch_picker, branch_picker_count, branch_picker_next, branch_picker_previous,
    },
    conversation::{conversation, conversation_content, conversation_empty_state},
    copy_button::copy_button,
    loader::{LoaderVariant, loader},
    markdown::markdown,
    message::{MessageRole, message, message_action, message_actions, message_content},
    prompt_input::{
        PromptStatus, prompt_input, prompt_input_button, prompt_input_submit,
        prompt_input_textarea, prompt_input_toolbar, prompt_input_tools,
    },
    reasoning::{reasoning, reasoning_content},
    sources::{source, sources, sources_content},
    suggestion::{suggestion, suggestions},
    tool_call::{ToolStatus, tool_call, tool_call_input, tool_call_output},
};

/// Starter prompts shown on the empty conversation.
const SUGGESTIONS: [&str; 3] = [
    "How do live regions stream updates?",
    "Show me a Rust function that reverses words",
    "What can the fake model do?",
];

/// The longest message the demo accepts, in bytes.
const MAX_MESSAGE_LEN: usize = 4000;

/// The chat: the conversation above, the composer below. It fills its container, so
/// give that a height.
#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let draft = signal(cx, String::new);

    Ok(view! {
        <div class="flex h-full min-h-0 w-full flex-1 flex-col">
            chat_view(draft: draft)
        </div>
    })
}

// The HTTP response renders the conversation as it is and finishes. Calling
// connected() makes the browser open a WebSocket and render the shard again; that
// render waits for changes and replaces the region after each one.
#[shard]
async fn chat_view(cx: &Cx, draft: Signal<String>) -> Result<impl View> {
    Ok(live! {
        let chat = context::<Chat>(cx);
        let mut changed = chat.subscribe();
        loop {
            let snapshot = chat.snapshot();
            let token = emit! { chat_panel(snapshot: snapshot, draft: draft.clone()) }?;
            if !connected(cx) {
                break Ok(token);
            }
            // A lagged receiver only means updates were coalesced; render again.
            if let Err(broadcast::error::RecvError::Closed) = changed.recv().await {
                break Ok(token);
            }
        }
    })
}

/// The conversation and the composer for one snapshot of the chat.
#[component]
async fn chat_panel(cx: &Cx, snapshot: Snapshot, draft: Signal<String>) -> Result<impl View> {
    let status = snapshot.status;
    let busy = status != PromptStatus::Ready;
    let last = snapshot.messages.len();

    Ok(view! {
        conversation(
            attrs: attributes! { aria-busy=(if busy { "true" } else { "false" }) },
            conversation_content(
                if snapshot.messages.is_empty() {
                    conversation_empty_state(
                        title: "Chat with a fake model",
                        description: "Responses are canned and streamed by a Tokio task: \
                            reasoning, a tool call, then markdown with code and sources.",
                        suggestions(
                            attrs: attributes! { class="max-w-md justify-center" },
                            for prompt in SUGGESTIONS {
                                suggestion(
                                    prompt: prompt,
                                    attrs: attributes! {
                                        @click=$(async |e: Event| {
                                            send(e.current_target.value).await;
                                        })
                                    }
                                )
                            }
                        )
                        <a
                            href="/full/claude"
                            class="text-sm text-muted-foreground underline underline-offset-4 hover:text-foreground"
                        >
                            "Try it with the real Claude API, on your own key"
                        </a>
                    )
                }
                #[key(entry.id)]
                for (index, entry) in snapshot.messages.into_iter().enumerate() {
                    chat_message(entry: entry, last: index + 1 == last)
                }
            )
        )
        <div class="mx-auto w-full max-w-3xl shrink-0 px-4 pb-4">
            prompt_input(
                attrs: attributes! {
                    @submit=$(async |e: Event| {
                        e.prevent_default();
                        let accepted = send(draft.get()).await;
                        if accepted {
                            draft.set("".to_owned());
                        }
                    })
                },
                prompt_input_textarea(
                    attrs: attributes! {
                        name="message"
                        placeholder="Send a message…"
                        :value=$(draft.get())
                        @input=$(|e: Event| draft.set(e.target.value))
                    }
                )
                prompt_input_toolbar(
                    prompt_input_tools(
                        prompt_input_button(
                            attrs: attributes! {
                                aria-label="Clear conversation"
                                title="Clear conversation"
                                disabled=(busy)
                                @click=$(async |_e: Event| {
                                    reset().await;
                                })
                            },
                            icon(data: iconify_icon!("lucide:trash-2"))
                        )
                        <span class="px-1 text-xs text-muted-foreground max-sm:hidden">
                            "Enter to send, Shift+Enter for a new line"
                        </span>
                    )
                    // While a response is on its way, the button is a plain
                    // button that stops it.
                    prompt_input_submit(
                        status: status,
                        disabled: $(draft.get().trim().is_empty()),
                        attrs: attributes! {
                            if busy {
                                @click=$(async |_e: Event| {
                                    stop().await;
                                })
                            }
                        }
                    )
                )
            )
        </div>
    })
}

/// One message of the conversation.
#[component]
async fn chat_message(cx: &Cx, entry: Entry, last: bool) -> Result<impl View> {
    Ok(view! {
        match entry.body {
            Body::User(text) => {
                message(role: MessageRole::User, message_content((text)))
            }
            Body::Assistant { branches, current } => {
                let total = branches.len();
                let response = &branches[current];
                let id = entry.id;
                message(
                    role: MessageRole::Assistant,
                    message_content(assistant_response(response: response.clone()))
                    if response.finished() {
                        message_actions(
                            if total > 1 {
                                branch_picker(
                                    branch_picker_previous(
                                        attrs: attributes! {
                                            disabled=(current == 0)
                                            @click=$(async |_e: Event| {
                                                select_branch(id, current - 1).await;
                                            })
                                        }
                                    )
                                    branch_picker_count(current: current + 1, total: total)
                                    branch_picker_next(
                                        attrs: attributes! {
                                            disabled=(current + 1 == total)
                                            @click=$(async |_e: Event| {
                                                select_branch(id, current + 1).await;
                                            })
                                        }
                                    )
                                )
                            }
                            copy_button(text: &response.answer, label: "Copy message")
                            if last {
                                message_action(
                                    label: "Regenerate",
                                    attrs: attributes! {
                                        @click=$(async |_e: Event| {
                                            regenerate().await;
                                        })
                                    },
                                    icon(data: iconify_icon!("lucide:refresh-cw"))
                                )
                            }
                        )
                    }
                )
            }
        }
    })
}

/// The parts of an assistant response, in the order the model produced them.
#[component]
async fn assistant_response(cx: &Cx, response: Response) -> Result<impl View> {
    Ok(view! {
        if response.is_waiting() {
            loader(variant: LoaderVariant::Dots)
        }
        if let Some(thought) = &response.reasoning {
            reasoning(
                streaming: thought.streaming,
                duration_secs: thought.duration_secs,
                reasoning_content((thought.text.clone()))
            )
        }
        if let Some(tool) = &response.tool {
            tool_call(
                name: tool.name,
                status: tool.status,
                tool_call_input((tool.input))
                if let Some(output) = tool.output {
                    tool_call_output((output))
                }
            )
        }
        if !response.answer.is_empty() {
            markdown(source: &response.answer)
        }
        if !response.sources.is_empty() {
            sources(
                count: response.sources.len(),
                sources_content(
                    for (title, href) in &response.sources {
                        source(href: href, title: title)
                    }
                )
            )
        }
        if response.state == ResponseState::Stopped {
            <p class="text-xs text-muted-foreground">"Stopped."</p>
        }
    })
}

/// Sends a user message and starts a response. Returns whether the message was
/// accepted: it is rejected if it is empty, too long, or a response is in progress.
#[procedure]
pub async fn send(cx: &Cx, text: String) -> Result<bool> {
    let text = text.trim();
    if text.is_empty() || text.len() > MAX_MESSAGE_LEN {
        return Ok(false);
    }
    Ok(context::<Chat>(cx).send(text.to_owned()))
}

/// Stops the response in progress, keeping what has arrived.
#[procedure]
pub async fn stop(cx: &Cx) -> Result<bool> {
    context::<Chat>(cx).stop();
    Ok(true)
}

/// Generates another response to the last message, kept as an alternate branch.
#[procedure]
pub async fn regenerate(cx: &Cx) -> Result<bool> {
    Ok(context::<Chat>(cx).regenerate())
}

/// Shows another branch of an assistant message.
#[procedure]
pub async fn select_branch(cx: &Cx, message: u64, branch: usize) -> Result<bool> {
    Ok(context::<Chat>(cx).select_branch(message, branch))
}

/// Clears the conversation.
#[procedure]
pub async fn reset(cx: &Cx) -> Result<bool> {
    context::<Chat>(cx).reset();
    Ok(true)
}

/// The shared conversation and the task streaming into it.
#[derive(Clone)]
pub struct Chat {
    inner: Arc<Inner>,
}

struct Inner {
    state: Mutex<State>,
    changed: broadcast::Sender<()>,
}

#[derive(Default)]
struct State {
    messages: Vec<Entry>,
    next_id: u64,
    /// The task streaming the current response, if one is running.
    generation: Option<AbortHandle>,
}

/// One message in the conversation.
#[derive(Clone)]
struct Entry {
    id: u64,
    body: Body,
}

#[derive(Clone)]
enum Body {
    User(String),
    /// Every response generated for the preceding user message, and the one shown.
    Assistant {
        branches: Vec<Response>,
        current: usize,
    },
}

/// One response from the fake model.
#[derive(Clone, Default)]
struct Response {
    state: ResponseState,
    reasoning: Option<Thought>,
    tool: Option<Tool>,
    answer: String,
    sources: Vec<(&'static str, &'static str)>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ResponseState {
    /// Sent, nothing has arrived yet.
    #[default]
    Submitted,
    Streaming,
    Done,
    Stopped,
}

#[derive(Clone)]
struct Thought {
    text: String,
    streaming: bool,
    duration_secs: Option<u32>,
}

#[derive(Clone)]
struct Tool {
    name: &'static str,
    status: ToolStatus,
    input: &'static str,
    output: Option<&'static str>,
}

impl Response {
    fn finished(&self) -> bool {
        matches!(self.state, ResponseState::Done | ResponseState::Stopped)
    }

    /// Whether the response has nothing to show yet.
    fn is_waiting(&self) -> bool {
        self.state == ResponseState::Submitted
    }
}

/// What the page renders: the messages and the composer's status.
pub struct Snapshot {
    messages: Vec<Entry>,
    status: PromptStatus,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(State::default()),
                changed: broadcast::channel(64).0,
            }),
        }
    }
}

impl Chat {
    /// Call before reading the conversation, so a change between the read and the
    /// wait is not missed.
    fn subscribe(&self) -> broadcast::Receiver<()> {
        self.inner.changed.subscribe()
    }

    fn notify(&self) {
        // No subscribers is fine: nobody is watching.
        let _ = self.inner.changed.send(());
    }

    fn snapshot(&self) -> Snapshot {
        let state = self.inner.state.lock().unwrap();
        let status = match state.messages.last().map(|entry| &entry.body) {
            Some(Body::Assistant { branches, current }) if state.generation.is_some() => {
                if branches[*current].is_waiting() {
                    PromptStatus::Submitted
                } else {
                    PromptStatus::Streaming
                }
            }
            _ => PromptStatus::Ready,
        };
        Snapshot {
            messages: state.messages.clone(),
            status,
        }
    }

    /// Adds a user message and starts answering it. Returns `false` while another
    /// response is in progress.
    fn send(&self, text: String) -> bool {
        let mut state = self.inner.state.lock().unwrap();
        if state.generation.is_some() {
            return false;
        }
        let prompt = text.clone();
        state.push(Body::User(text));
        state.push(Body::Assistant {
            branches: vec![Response::default()],
            current: 0,
        });
        state.generation = Some(self.spawn(prompt));
        drop(state);
        self.notify();
        true
    }

    /// Starts another response to the last user message as a new branch of the last
    /// assistant message.
    fn regenerate(&self) -> bool {
        let mut state = self.inner.state.lock().unwrap();
        if state.generation.is_some() {
            return false;
        }
        let Some(prompt) = state
            .messages
            .iter()
            .rev()
            .find_map(|entry| match &entry.body {
                Body::User(text) => Some(text.clone()),
                Body::Assistant { .. } => None,
            })
        else {
            return false;
        };
        let Some(Body::Assistant { branches, current }) =
            state.messages.last_mut().map(|entry| &mut entry.body)
        else {
            return false;
        };
        branches.push(Response::default());
        *current = branches.len() - 1;
        state.generation = Some(self.spawn(prompt));
        drop(state);
        self.notify();
        true
    }

    fn select_branch(&self, message: u64, branch: usize) -> bool {
        let mut state = self.inner.state.lock().unwrap();
        // Switching while a branch streams would hide the stream.
        if state.generation.is_some() {
            return false;
        }
        let Some(Body::Assistant { branches, current }) = state
            .messages
            .iter_mut()
            .find(|entry| entry.id == message)
            .map(|entry| &mut entry.body)
        else {
            return false;
        };
        if branch >= branches.len() {
            return false;
        }
        *current = branch;
        drop(state);
        self.notify();
        true
    }

    fn stop(&self) {
        let mut state = self.inner.state.lock().unwrap();
        let Some(generation) = state.generation.take() else {
            return;
        };
        generation.abort();
        if let Some(response) = state.current_response() {
            response.state = ResponseState::Stopped;
            if let Some(thought) = &mut response.reasoning {
                thought.streaming = false;
            }
            if let Some(tool) = &mut response.tool
                && matches!(tool.status, ToolStatus::Pending | ToolStatus::Running)
            {
                tool.status = ToolStatus::Error;
                tool.output = Some("Cancelled.");
            }
        }
        drop(state);
        self.notify();
    }

    fn reset(&self) {
        let mut state = self.inner.state.lock().unwrap();
        if let Some(generation) = state.generation.take() {
            generation.abort();
        }
        state.messages.clear();
        drop(state);
        self.notify();
    }

    /// Applies `change` to the response being generated, then notifies observers.
    fn update(&self, change: impl FnOnce(&mut Response)) {
        let mut state = self.inner.state.lock().unwrap();
        if let Some(response) = state.current_response() {
            change(response);
        }
        drop(state);
        self.notify();
    }

    /// Spawns the fake model, streaming into the shown branch of the last message.
    fn spawn(&self, prompt: String) -> AbortHandle {
        let chat = self.clone();
        tokio::spawn(async move {
            fake_model(&chat, &prompt).await;
            let mut state = chat.inner.state.lock().unwrap();
            if let Some(response) = state.current_response() {
                response.state = ResponseState::Done;
            }
            state.generation = None;
            drop(state);
            chat.notify();
        })
        .abort_handle()
    }
}

impl State {
    fn push(&mut self, body: Body) {
        let id = self.next_id;
        self.next_id += 1;
        self.messages.push(Entry { id, body });
    }

    /// The branch being generated: the shown branch of the last message.
    fn current_response(&mut self) -> Option<&mut Response> {
        match self.messages.last_mut().map(|entry| &mut entry.body) {
            Some(Body::Assistant { branches, current }) => branches.get_mut(*current),
            _ => None,
        }
    }
}

/// Streams a canned response: a reasoning phase, a tool call that runs and completes,
/// then markdown with a code block, and finally sources.
async fn fake_model(chat: &Chat, prompt: &str) {
    let script = Script::for_prompt(prompt);

    // The gap before the first token, shown as a typing indicator.
    sleep(Duration::from_millis(700)).await;

    let started = Instant::now();
    chat.update(|response| {
        response.state = ResponseState::Streaming;
        response.reasoning = Some(Thought {
            text: String::new(),
            streaming: true,
            duration_secs: None,
        });
    });
    for chunk in chunks(&script.reasoning, 3) {
        sleep(Duration::from_millis(60)).await;
        chat.update(|response| {
            if let Some(thought) = &mut response.reasoning {
                thought.text.push_str(chunk);
            }
        });
    }
    let seconds = started.elapsed().as_secs_f32().round() as u32;
    chat.update(|response| {
        if let Some(thought) = &mut response.reasoning {
            thought.streaming = false;
            thought.duration_secs = Some(seconds.max(1));
        }
        response.tool = Some(Tool {
            name: script.tool_name,
            status: ToolStatus::Pending,
            input: script.tool_input,
            output: None,
        });
    });

    sleep(Duration::from_millis(400)).await;
    chat.update(|response| {
        if let Some(tool) = &mut response.tool {
            tool.status = ToolStatus::Running;
        }
    });
    sleep(Duration::from_millis(1500)).await;
    chat.update(|response| {
        if let Some(tool) = &mut response.tool {
            tool.status = ToolStatus::Completed;
            tool.output = Some(script.tool_output);
        }
    });

    for chunk in chunks(&script.answer, 4) {
        sleep(Duration::from_millis(35)).await;
        chat.update(|response| response.answer.push_str(chunk));
    }

    sleep(Duration::from_millis(200)).await;
    chat.update(|response| response.sources = script.sources.to_vec());
}

/// Splits `text` into chunks of about `words` words, keeping the whitespace, the way
/// tokens arrive from a model.
fn chunks(text: &str, words: usize) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut seen = 0;
    let mut in_word = false;
    for (index, c) in text.char_indices() {
        if c.is_whitespace() {
            if in_word {
                seen += 1;
                in_word = false;
                if seen == words {
                    chunks.push(&text[start..index]);
                    start = index;
                    seen = 0;
                }
            }
        } else {
            in_word = true;
        }
    }
    if start < text.len() {
        chunks.push(&text[start..]);
    }
    chunks
}

/// One canned response.
struct Script {
    reasoning: String,
    tool_name: &'static str,
    tool_input: &'static str,
    tool_output: &'static str,
    answer: String,
    sources: &'static [(&'static str, &'static str)],
}

impl Script {
    fn for_prompt(prompt: &str) -> Self {
        let prompt = prompt.to_lowercase();
        if prompt.contains("rust") || prompt.contains("code") || prompt.contains("function") {
            Self::code(&prompt)
        } else {
            Self::general(&prompt)
        }
    }

    fn code(prompt: &str) -> Self {
        Self {
            reasoning: format!(
                "The user asked: \"{prompt}\". A short, idiomatic answer works best. I should \
                 check the standard library docs for `split_whitespace` and `rev`, then show a \
                 function with a test, and point to the docs I used."
            ),
            tool_name: "search_docs",
            tool_input: "{\n  \"query\": \"str::split_whitespace\",\n  \"crate\": \"std\"\n}",
            tool_output: "[\n  { \"item\": \"str::split_whitespace\", \"kind\": \"method\" },\n  \
                          { \"item\": \"Iterator::rev\", \"kind\": \"method\" }\n]",
            answer: String::from(
                "Here is a function that reverses the order of words. It splits on any \
                 whitespace, so runs of spaces collapse to one:\n\n\
                 ```rust\n\
                 fn reverse_words(text: &str) -> String {\n    \
                     text.split_whitespace().rev().collect::<Vec<_>>().join(\" \")\n\
                 }\n\n\
                 #[test]\n\
                 fn reverses() {\n    \
                     assert_eq!(reverse_words(\"hello  big world\"), \"world big hello\");\n\
                 }\n\
                 ```\n\n\
                 A few notes:\n\n\
                 - `split_whitespace` skips empty pieces, unlike `split(' ')`.\n\
                 - `rev` works because the iterator is **double-ended**.\n\
                 - To keep the original spacing, split with `split_inclusive` instead.\n\n\
                 | Approach | Keeps spacing | Allocates |\n\
                 | --- | --- | --- |\n\
                 | `split_whitespace` | no | one `Vec` |\n\
                 | `split_inclusive` | yes | one `Vec` |\n\n\
                 See the [`str` docs](https://doc.rust-lang.org/std/primitive.str.html) for more.",
            ),
            sources: &[
                (
                    "str::split_whitespace - Rust standard library",
                    "https://doc.rust-lang.org/std/primitive.str.html#method.split_whitespace",
                ),
                (
                    "DoubleEndedIterator - Rust standard library",
                    "https://doc.rust-lang.org/std/iter/trait.DoubleEndedIterator.html",
                ),
            ],
        }
    }

    fn general(prompt: &str) -> Self {
        Self {
            reasoning: format!(
                "The user asked: \"{prompt}\". This is a demo, so I should explain what is \
                 happening on screen: the reasoning streams first, then a tool call, then this \
                 answer. Let me look up how Topcoat streams live regions."
            ),
            tool_name: "fetch_page",
            tool_input: "{\n  \"url\": \"https://docs.rs/topcoat/latest/topcoat/view/macro.live.html\"\n}",
            tool_output: "{\n  \"title\": \"live in topcoat::view\",\n  \"status\": 200\n}",
            answer: String::from(
                "## How this response reached you\n\n\
                 Nothing here comes from a real model. A **Tokio task** on the server appends \
                 canned text to the conversation in small chunks, and the page's `live!` region \
                 renders the whole conversation again after every chunk:\n\n\
                 1. You sent a message through a `#[procedure]`.\n\
                 2. The procedure spawned the task and returned right away.\n\
                 3. A `#[shard]` holding the live region waits on a broadcast channel and emits \
                 the new markup over a WebSocket.\n\n\
                 > The task runs on its own, so reloading the page or losing the connection \
                 picks the stream up where it is instead of starting over.\n\n\
                 The region is replaced on each update, but the browser morphs it in place, so \
                 your draft, focus, scroll position, and open sections survive. Try opening \
                 the reasoning above while this streams.\n\n\
                 ```sh\n\
                 topcoat ui add --all --registry topcoat-prompt\n\
                 ```\n\n\
                 - [x] Reasoning\n\
                 - [x] Tool call\n\
                 - [ ] A real model (bring your own)",
            ),
            sources: &[
                (
                    "Streaming regions - Topcoat",
                    "https://docs.rs/topcoat/latest/topcoat/view/macro.live.html",
                ),
                (
                    "Procedures - Topcoat",
                    "https://docs.rs/topcoat/latest/topcoat/runtime/attr.procedure.html",
                ),
                (
                    "Tokio tasks",
                    "https://docs.rs/tokio/latest/tokio/task/index.html",
                ),
            ],
        }
    }
}
