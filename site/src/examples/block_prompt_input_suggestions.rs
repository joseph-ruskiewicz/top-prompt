use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    prompt_input::{
        prompt_input, prompt_input_submit, prompt_input_textarea, prompt_input_toolbar,
        prompt_input_tools,
    },
    suggestion::{suggestion, suggestions},
};

const STARTERS: [&str; 4] = [
    "Plan a three-day trip to Kyoto",
    "Explain compound interest simply",
    "Draft a polite rent reminder",
    "Suggest a 20-minute dinner",
];

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let draft = signal(cx, String::new);

    Ok(view! {
        <div class="flex h-full w-full flex-col items-center justify-end gap-4 p-6">
            suggestions(
                attrs: attributes! { class="max-w-2xl justify-center" },
                for prompt in STARTERS {
                    // A click fills the composer; the reader can edit before sending.
                    suggestion(
                        prompt: prompt,
                        attrs: attributes! { @click=$(|e: Event| draft.set(e.current_target.value)) }
                    )
                }
            )
            prompt_input(
                attrs: attributes! {
                    class="max-w-2xl"
                    @submit=$(|e: Event| {
                        e.prevent_default();
                        draft.set("".to_owned());
                    })
                },
                prompt_input_textarea(
                    attrs: attributes! {
                        placeholder="Ask anything"
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
    })
}
