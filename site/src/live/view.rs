//! The real-model chat page: a key form, then the conversation.
//!
//! The page reads the visitor's session from the `__Host-demo` cookie, which the
//! `/full/live` layer sets on the first load. Procedures look the visitor up the
//! same way. The key reaches the server once, in the body of the `save_key`
//! procedure call; it is never put in a signal, so it is never rendered into the
//! page or sent back in a runtime message.

use topcoat::{
    Result,
    context::{Cx, context},
    icon::{icon, iconify::iconify_icon},
    router::request::client_ip,
    runtime::{Event, Signal, connected, procedure, shard, signal},
    session,
    view::{View, attributes, component, emit, live, view},
};

use super::{
    api::MAX_TOKENS,
    chat::{
        Demo, IDLE_TIMEOUT, MAX_MESSAGE_LEN, MAX_TURNS, Notice, Part, ReplyState, ReplyView,
        Snapshot, TurnView, VISITOR_REQUESTS, Visitor,
    },
    key,
    provider::{Model, Provider},
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
    suggestion::{suggestion, suggestions},
    system_message::{SystemMessageVariant, system_message, system_message_cta},
    tool_call::{tool_call, tool_call_input, tool_call_output},
};

const SUGGESTIONS: [&str; 3] = [
    "What is 1234 * 5678 / 9, exactly?",
    "Which topcoat-prompt components show a model's reasoning?",
    "Write a Rust function that checks for a palindrome.",
];

/// The whole page.
#[component]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let draft = signal(cx, String::new);
    let key_error = signal(cx, String::new);

    Ok(view! {
        <div class="flex h-full min-h-0 w-full flex-1 flex-col">
            demo_view(draft: draft, key_error: key_error)
        </div>
    })
}

/// The visitor's view, rendered again after every change to their state.
#[shard]
async fn demo_view(cx: &Cx, draft: Signal<String>, key_error: Signal<String>) -> Result<impl View> {
    Ok(live! {
        let demo = context::<Demo>(cx);
        let session = session::token_hash(cx).await?;
        loop {
            let visitor = session.as_ref().and_then(|session| demo.visitor(session));
            let mut changed = visitor.as_ref().map(|visitor| visitor.subscribe());
            let snapshot = visitor.as_ref().map(|visitor| visitor.snapshot());
            drop(visitor);
            let has_session = session.is_some();
            let token = emit! {
                demo_panel(
                    snapshot: snapshot,
                    has_session: has_session,
                    draft: draft.clone(),
                    key_error: key_error.clone()
                )
            }?;
            let Some(changed) = changed.as_mut() else {
                break Ok(token);
            };
            if !connected(cx) {
                break Ok(token);
            }
            // Lagging only means updates were coalesced, and a closed channel means
            // the visitor was dropped for being idle; either way, render again.
            let _ = changed.recv().await;
        }
    })
}

/// What a visitor sees for one snapshot.
#[component]
async fn demo_panel(
    cx: &Cx,
    snapshot: Option<Snapshot>,
    has_session: bool,
    draft: Signal<String>,
    key_error: Signal<String>,
) -> Result<impl View> {
    Ok(view! {
        match snapshot {
            Some(snapshot) if snapshot.has_key => chat_panel(
                snapshot: snapshot,
                draft: draft
            ),
            Some(snapshot) => key_form(
                provider: snapshot.provider,
                key_error: key_error
            ),
            None if has_session => <div class="m-auto max-w-md p-6">
                system_message(
                    variant: SystemMessageVariant::Warning,
                    "The demo has too many visitors right now. Try again in a few minutes."
                )
            </div>,
            None => <div class="m-auto max-w-md p-6">
                system_message(
                    variant: SystemMessageVariant::Warning,
                    "This page needs its session cookie."
                    system_message_cta(
                        attrs: attributes! { href="/full/live" },
                        "Reload"
                    )
                )
            </div>,
        }
    })
}

/// The provider choice and the form for the key, with what happens to it.
#[component]
async fn key_form(cx: &Cx, provider: Provider, key_error: Signal<String>) -> Result<impl View> {
    let idle_minutes = IDLE_TIMEOUT.as_secs() / 60;
    let name = provider.label();
    let placeholder = format!("{}…", key::prefix(provider));

    Ok(view! {
        <div class="min-h-0 flex-1 overflow-y-auto">
            <div class="mx-auto flex w-full max-w-xl flex-col gap-6 px-4 py-10">
                <div class="flex flex-col gap-2">
                    <h1 class="text-2xl font-[450] tracking-tight">
                        "Live model demo"
                    </h1>
                    <p class="text-sm leading-relaxed text-muted-foreground">
                        "The same components as the "
                        <a href="/full/chat-app" class="underline underline-offset-4">
                            "fake-model demo"
                        </a>
                        ", answered by a real model from Anthropic or OpenAI on your own API \
                         key: streamed text, summarized reasoning and real tool calls."
                    </p>
                </div>
                <form
                    class="flex flex-col gap-4"
                    @submit=$(|e: Event| e.prevent_default())
                >
                    // Native radios: Tab reaches the group, arrows move the choice.
                    <fieldset class="flex flex-col gap-2">
                        <legend class="mb-2 text-sm font-medium">"Provider"</legend>
                        <div
                            class="flex w-fit gap-0.5 rounded-lg border border-border p-0.5"
                        >
                            for option in Provider::ALL {
                                <label
                                    class="cursor-pointer rounded-md px-3 py-1 text-sm text-muted-foreground has-[:checked]:bg-foreground/10 has-[:checked]:font-medium has-[:checked]:text-foreground has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring hover:text-foreground"
                                >
                                    <input
                                        type="radio"
                                        name="provider"
                                        value=(option.id())
                                        checked=(option == provider)
                                        class="sr-only"
                                        @change=$(async |e: Event| {
                                            // A key typed for the old provider goes
                                            // too, so the next one is a change.
                                            raw!("document.getElementById('api-key').value = '';", ());
                                            key_error.set("".to_owned());
                                            set_provider(e.target.value).await;
                                        })
                                    >
                                    (option.label())
                                </label>
                            }
                        </div>
                    </fieldset>
                    <div class="flex flex-col gap-2">
                        <label for="api-key" class="text-sm font-medium">
                            (name)
                            " API key"
                        </label>
                        <div class="flex gap-2">
                            // The value goes to the server once, when the field changes; it
                            // is not kept in a signal. Password managers are asked not to
                            // save it.
                            <input
                                id="api-key"
                                type="password"
                                name="api-key"
                                autocomplete="off"
                                spellcheck="false"
                                data-1p-ignore="true"
                                data-lpignore="true"
                                placeholder=(placeholder)
                                aria-describedby="api-key-help"
                                class="h-9 min-w-0 flex-1 rounded-lg border border-border bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                                @change=$(async |e: Event| {
                                    key_error.set("Checking…".to_owned());
                                    let message = save_key(e.target.value).await;
                                    key_error.set(message);
                                })
                            >
                            <button
                                type="submit"
                                class="h-9 shrink-0 rounded-lg bg-primary px-3 text-sm font-medium text-primary-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
                            >
                                "Use key"
                            </button>
                        </div>
                        <p
                            id="api-key-help"
                            class="text-sm leading-relaxed text-muted-foreground"
                        >
                            "Use a key made for this: create one in "
                            <a
                                href=(provider.keys_url())
                                target="_blank"
                                rel="noreferrer"
                                class="underline underline-offset-4"
                            >
                                (provider.console())
                            </a>
                            " "
                            (provider.key_advice())
                            ", and delete it afterwards."
                        </p>
                        <p role="status" class="min-h-5 text-sm text-muted-foreground">
                            $(key_error.get())
                        </p>
                    </div>
                </form>
                <ul
                    class="flex list-disc flex-col gap-1.5 pl-5 text-sm leading-relaxed"
                >
                    <li>
                        "Your key is sent to the topcoat-prompt server, which calls the "
                        (name)
                        " API for you."
                    </li>
                    <li>
                        "It is held only in the server's memory and never stored: not on \
                         disk, not in a cookie, not in logs. It is dropped when you clear it, \
                         after "
                        (idle_minutes)
                        " minutes without activity, or when the server restarts."
                    </li>
                    <li>"It is used only for this demo."</li>
                    <li>
                        "Every message is billed to your "
                        (name)
                        " account."
                    </li>
                </ul>
                <p class="text-xs leading-relaxed text-muted-foreground">
                    "Limits: "
                    (MAX_TURNS)
                    " messages per conversation, "
                    (MAX_MESSAGE_LEN)
                    " characters per message, replies of up to "
                    (MAX_TOKENS)
                    " tokens, "
                    (VISITOR_REQUESTS)
                    " API requests per 10 minutes (each tool round trip is one)."
                </p>
            </div>
        </div>
    })
}

/// The conversation, the toolbar above it and the composer below it.
#[component]
async fn chat_panel(cx: &Cx, snapshot: Snapshot, draft: Signal<String>) -> Result<impl View> {
    let status = snapshot.status;
    let busy = status != PromptStatus::Ready;
    let full = snapshot.full;
    let last = snapshot.turns.len();
    let current_model = snapshot.model;
    let provider = snapshot.provider;
    // With two providers, changing means the other one.
    let other = if provider == Provider::Anthropic {
        Provider::OpenAi
    } else {
        Provider::Anthropic
    };
    let other_id = other.id().to_owned();

    Ok(view! {
        <header
            class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-2 border-b border-border px-4 py-2 text-sm"
        >
            <a
                href="/full/chat-app"
                class="inline-flex items-center gap-1 text-muted-foreground hover:text-foreground"
            >
                icon(data: iconify_icon!("lucide:arrow-left"))
                "Fake-model demo"
            </a>
            <span class="font-medium">(provider.label())</span>
            <select
                aria-label="Model"
                disabled=(busy)
                class="h-8 rounded-lg border border-border bg-background px-2 text-sm"
                @change=$(async |e: Event| {
                    set_model(e.target.value).await;
                })
            >
                for model in Model::for_provider(provider) {
                    <option value=(model.id()) selected=(model == current_model)>
                        (model.label())
                    </option>
                }
            </select>
            <span class="text-xs text-muted-foreground">
                "Billed to your "
                (provider.label())
                " account"
            </span>
            <div class="ml-auto flex items-center gap-1">
                // Both go back to the key form: one for the other provider, one for
                // this provider with a new key.
                <button
                    type="button"
                    class="inline-flex h-8 items-center gap-1.5 rounded-lg px-2 text-sm text-muted-foreground hover:bg-foreground/5 hover:text-foreground"
                    @click=$(async |_e: Event| {
                        set_provider(other_id.clone()).await;
                    })
                >
                    icon(data: iconify_icon!("lucide:arrow-left-right"))
                    "Change provider"
                </button>
                <button
                    type="button"
                    class="inline-flex h-8 items-center gap-1.5 rounded-lg px-2 text-sm text-muted-foreground hover:bg-foreground/5 hover:text-foreground"
                    @click=$(async |_e: Event| {
                        clear_key().await;
                    })
                >
                    icon(data: iconify_icon!("lucide:key-round"))
                    "Clear key"
                </button>
            </div>
        </header>
        conversation(
            attrs: attributes! { aria-busy=(if busy { "true" } else { "false" }) },
            conversation_content(
                if snapshot.turns.is_empty() {
                    conversation_empty_state(
                        title: format!("Ask {} something", current_model.label()),
                        description: format!(
                            "Replies stream from the {} API with the model's reasoning \
                             summarized. The model can call two tools here: a calculator and \
                             a search of this site's components.",
                            provider.label(),
                        ),
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
                    )
                }
                #[key(turn.id)]
                for (index, turn) in snapshot.turns.into_iter().enumerate() {
                    turn_view(turn: turn, last: index + 1 == last)
                }
                if full {
                    system_message(
                        variant: SystemMessageVariant::Warning,
                        "This conversation reached the demo's limit of "
                        (MAX_TURNS)
                        " messages."
                        system_message_cta(
                            attrs: attributes! {
                                @click=$(async |_e: Event| {
                                    reset().await;
                                })
                            },
                            "Start over"
                        )
                    )
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
                        placeholder=(format!("Message {}…", current_model.label()))
                        maxlength=(MAX_MESSAGE_LEN)
                        disabled=(full)
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
                            (current_model.label())
                        </span>
                    )
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

/// A user message and the shown reply to it.
#[component]
async fn turn_view(cx: &Cx, turn: TurnView, last: bool) -> Result<impl View> {
    let total = turn.replies.len();
    let current = turn.current;
    let id = turn.id;
    let reply = turn.replies[current].clone();
    let text = reply.text();

    Ok(view! {
        message(role: MessageRole::User, message_content((turn.user)))
        message(
            role: MessageRole::Assistant,
            message_content(reply_view(reply: reply.clone()))
            if reply.finished() {
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
                    if !text.is_empty() {
                        copy_button(text: &text, label: "Copy message")
                    }
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
                    <span class="px-1 text-xs text-muted-foreground">
                        (reply.model.label())
                    </span>
                )
            }
        )
    })
}

/// The parts of a reply in the order they streamed, then its notice.
#[component]
async fn reply_view(cx: &Cx, reply: ReplyView) -> Result<impl View> {
    Ok(view! {
        if reply.state == ReplyState::Waiting {
            loader(variant: LoaderVariant::Dots)
        }
        // Parts are only ever appended, so a part's index is its identity.
        #[key(index)]
        for (index, part) in reply.parts.into_iter().enumerate() {
            match part {
                Part::Thinking { text, streaming, secs, .. } => {
                    // Adaptive thinking may skip thinking, or stream an empty block.
                    if streaming || !text.trim().is_empty() {
                        reasoning(
                            streaming: streaming,
                            duration_secs: secs,
                            reasoning_content((text))
                        )
                    }
                }
                Part::Tool { name, status, input, output, error } => {
                    tool_call(
                        name: name,
                        status: status,
                        tool_call_input((input))
                        if let Some(output) = output {
                            tool_call_output(error: error, (output))
                        }
                    )
                }
                Part::Text(text) => {
                    if !text.is_empty() {
                        markdown(source: &text)
                    }
                }
            }
        }
        if reply.state == ReplyState::Stopped {
            <p class="text-xs text-muted-foreground">"Stopped."</p>
        }
        if let Some(notice) = reply.notice {
            notice_view(notice: notice)
        }
    })
}

#[component]
async fn notice_view(cx: &Cx, notice: Notice) -> Result<impl View> {
    Ok(view! {
        system_message(
            variant: if notice.error {
                SystemMessageVariant::Error
            } else {
                SystemMessageVariant::Warning
            },
            fill: true,
            (notice.text)
            if notice.key_problem {
                system_message_cta(
                    attrs: attributes! {
                        @click=$(async |_e: Event| {
                            clear_key().await;
                        })
                    },
                    "Change key"
                )
            }
        )
    })
}

/// The visitor for the request's session.
async fn visitor(cx: &Cx) -> Result<Option<std::sync::Arc<Visitor>>> {
    let session = session::token_hash(cx).await?;
    Ok(session.and_then(|session| context::<Demo>(cx).visitor(&session)))
}

/// Stores the visitor's key. Returns a message for the page: empty when accepted.
#[procedure]
pub async fn save_key(cx: &Cx, key: String) -> Result<String> {
    let Some(visitor) = visitor(cx).await? else {
        return Ok("This page lost its session. Reload it and try again.".to_owned());
    };
    Ok(match visitor.set_key(&key) {
        Ok(()) => String::new(),
        Err(message) => message.to_owned(),
    })
}

/// Forgets the key and the conversation.
#[procedure]
pub async fn clear_key(cx: &Cx) -> Result<bool> {
    if let Some(visitor) = visitor(cx).await? {
        visitor.clear_key();
    }
    Ok(true)
}

/// Switches provider, from the listed ones only. This forgets the key and the
/// conversation.
#[procedure]
pub async fn set_provider(cx: &Cx, provider: String) -> Result<bool> {
    let (Some(visitor), Some(provider)) = (visitor(cx).await?, Provider::from_id(&provider)) else {
        return Ok(false);
    };
    Ok(visitor.set_provider(provider))
}

/// Picks the model for the next replies, from the allowed list only.
#[procedure]
pub async fn set_model(cx: &Cx, model: String) -> Result<bool> {
    let (Some(visitor), Some(model)) = (visitor(cx).await?, Model::from_id(&model)) else {
        return Ok(false);
    };
    Ok(visitor.set_model(model))
}

/// Sends a message and starts a reply.
#[procedure]
pub async fn send(cx: &Cx, text: String) -> Result<bool> {
    let Some(visitor) = visitor(cx).await? else {
        return Ok(false);
    };
    Ok(visitor.send(context::<Demo>(cx), &text, client_ip(cx)))
}

/// Stops the reply in progress, keeping what has arrived.
#[procedure]
pub async fn stop(cx: &Cx) -> Result<bool> {
    if let Some(visitor) = visitor(cx).await? {
        visitor.stop();
    }
    Ok(true)
}

/// Generates another reply to the last message, as a new branch.
#[procedure]
pub async fn regenerate(cx: &Cx) -> Result<bool> {
    let Some(visitor) = visitor(cx).await? else {
        return Ok(false);
    };
    Ok(visitor.regenerate(context::<Demo>(cx), client_ip(cx)))
}

/// Shows another reply to a message.
#[procedure]
pub async fn select_branch(cx: &Cx, turn: u64, branch: usize) -> Result<bool> {
    let Some(visitor) = visitor(cx).await? else {
        return Ok(false);
    };
    Ok(visitor.select_branch(turn, branch))
}

/// Clears the conversation, keeping the key.
#[procedure]
pub async fn reset(cx: &Cx) -> Result<bool> {
    if let Some(visitor) = visitor(cx).await? {
        visitor.reset();
    }
    Ok(true)
}
