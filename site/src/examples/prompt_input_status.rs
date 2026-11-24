use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::prompt_input::{
    PromptStatus, prompt_input, prompt_input_submit, prompt_input_textarea, prompt_input_toolbar,
    prompt_input_tools,
};

/// Stands in for sending the message: waits for the model to accept it. `fail` lets the
/// example show the error state.
#[procedure]
async fn send(cx: &Cx, message: String, fail: bool) -> Result<bool> {
    tokio::time::sleep(Duration::from_millis(1200)).await;
    Ok(!fail && !message.trim().is_empty())
}

/// Stands in for the reply streaming in.
#[procedure]
async fn stream_reply(cx: &Cx) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(2500)).await;
    Ok(())
}

/// The composer re-renders with each status change, because `status` is a server prop.
#[shard]
async fn composer(cx: &Cx) -> Result<impl View> {
    let draft = signal(cx, String::new);
    // 0 ready, 1 submitted, 2 streaming, 3 error.
    let status = signal(cx, || 0usize);
    let fail = signal(cx, || false);
    let (prompt_status, note) = match status.get() {
        1 => (
            PromptStatus::Submitted,
            "Submitted: waiting for the first token. Stop cancels.",
        ),
        2 => (
            PromptStatus::Streaming,
            "Streaming the reply. Stop cancels.",
        ),
        3 => (
            PromptStatus::Error,
            "The request failed. The button now retries it.",
        ),
        _ => (
            PromptStatus::Ready,
            "Ready. Type something and press Enter.",
        ),
    };

    Ok(view! {
        <div class="flex w-full flex-col gap-2">
            prompt_input(
                attrs: attributes! {
                    @submit=$(async |e: Event| {
                        e.prevent_default();
                        status.set(1);
                        let accepted = send(draft.get(), fail.get()).await;
                        // Stop may have been pressed while waiting.
                        if status.get() == 1 {
                            status.set(if accepted { 2 } else { 3 });
                        }
                        stream_reply().await;
                        if status.get() == 2 {
                            status.set(0);
                            draft.set("".to_owned());
                        }
                    })
                },
                prompt_input_textarea(
                    attrs: attributes! {
                        placeholder="Ask for a packing list…"
                        :value=$(draft.get())
                        @input=$(|e: Event| draft.set(e.target.value))
                    }
                )
                prompt_input_toolbar(
                    prompt_input_tools(
                        <label
                            class="flex items-center gap-2 px-2 text-xs text-muted-foreground"
                        >
                            <input
                                type="checkbox"
                                :checked=$(fail.get())
                                @change=$(|_e: Event| fail.toggle())
                            >
                            "Fail the next request"
                        </label>
                    )
                    prompt_input_submit(
                        status: prompt_status,
                        // Applies while ready or failed: nothing to send yet.
                        disabled: $(draft.get().trim().is_empty()),
                        // Submitted and Streaming make it a plain Stop button.
                        attrs: attributes! {
                            @click=$(|_e: Event| {
                                if status.get() == 1 {
                                    status.set(0)
                                } else if status.get() == 2 {
                                    status.set(0)
                                }
                            })
                        }
                    )
                )
            )
            <p role="status" class="text-xs text-muted-foreground">(note)</p>
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { composer() })
}
