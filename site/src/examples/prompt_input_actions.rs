use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, procedure, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::ButtonSize,
    prompt_input::{
        PromptStatus, prompt_input, prompt_input_button, prompt_input_submit,
        prompt_input_textarea, prompt_input_toolbar, prompt_input_tools,
    },
};

/// Stands in for sending the message to a model.
#[procedure]
async fn pretend_to_send(cx: &Cx, message: String) -> Result<bool> {
    tokio::time::sleep(Duration::from_millis(1500 + 10 * message.len() as u64)).await;
    Ok(true)
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let draft = signal(cx, String::new);
    let sending = signal(cx, || false);

    Ok(view! {
        prompt_input(
            attrs: attributes! {
                class="w-full"
                @submit=$(async |e: Event| {
                    e.prevent_default();
                    sending.set(true);
                    let message = draft.get();
                    draft.set("".to_owned());
                    pretend_to_send(message).await;
                    sending.set(false);
                })
            },
            prompt_input_textarea(
                attrs: attributes! {
                    placeholder="Plan a weekend in Lisbon…"
                    :value=$(draft.get())
                    @input=$(|e: Event| draft.set(e.target.value))
                }
            )
            prompt_input_toolbar(
                prompt_input_tools(
                    prompt_input_button(
                        attrs: attributes! { aria-label="Attach files" },
                        icon(data: iconify_icon!("lucide:plus"))
                    )
                    prompt_input_button(
                        size: ButtonSize::Sm,
                        attrs: attributes! { class="text-muted-foreground" },
                        icon(data: iconify_icon!("lucide:globe"))
                        "Search"
                    )
                    prompt_input_button(
                        attrs: attributes! { aria-label="More tools" },
                        icon(data: iconify_icon!("lucide:ellipsis"))
                    )
                )
                // Two buttons, one shown at a time: the status is a server-side prop,
                // and this composer switches in the browser.
                <span class="contents" :hidden=$(sending.get())>
                    prompt_input_submit(disabled: $(draft.get().trim().is_empty()))
                </span>
                <span class="contents" :hidden=$(!sending.get())>
                    prompt_input_submit(status: PromptStatus::Streaming)
                </span>
            )
        )
    })
}
