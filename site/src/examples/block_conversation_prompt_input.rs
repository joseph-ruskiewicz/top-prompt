use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    conversation::{conversation, conversation_content},
    message::{MessageRole, message, message_content},
    prompt_input::{
        prompt_input, prompt_input_submit, prompt_input_textarea, prompt_input_toolbar,
        prompt_input_tools,
    },
};

const MESSAGES: [(MessageRole, &str); 4] = [
    (MessageRole::User, "Give me a name for a bakery cat."),
    (
        MessageRole::Assistant,
        "Crumb. Or Brioche, if it is a fluffy one.",
    ),
    (MessageRole::User, "It is very fluffy."),
    (MessageRole::Assistant, "Then Brioche it is."),
];

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let draft = signal(cx, String::new);

    Ok(view! {
        <div class="flex h-full w-full flex-col">
            conversation(
                conversation_content(
                    for (role, text) in MESSAGES {
                        message(role: role, message_content((text)))
                    }
                )
            )
            <div class="mx-auto w-full max-w-3xl shrink-0 px-4 pb-4">
                prompt_input(
                    attrs: attributes! {
                        @submit=$(|e: Event| {
                            // Send the draft to your server, which adds it to the log.
                            e.prevent_default();
                            draft.set("".to_owned());
                        })
                    },
                    prompt_input_textarea(
                        attrs: attributes! {
                            placeholder="Reply…"
                            :value=$(draft.get())
                            @input=$(|e: Event| draft.set(e.target.value))
                        }
                    )
                    prompt_input_toolbar(
                        prompt_input_tools()
                        prompt_input_submit(disabled: $(draft.get().trim().is_empty()))
                    )
                )
            </div>
        </div>
    })
}
