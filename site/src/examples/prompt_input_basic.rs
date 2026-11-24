use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::prompt_input::{
    prompt_input, prompt_input_submit, prompt_input_textarea, prompt_input_toolbar,
    prompt_input_tools,
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let draft = signal(cx, String::new);

    Ok(view! {
        prompt_input(
            attrs: attributes! {
                @submit=$(|e: Event| {
                    // Send the draft to your server here, for example with a procedure.
                    e.prevent_default();
                    draft.set("".to_owned());
                })
            },
            prompt_input_textarea(
                attrs: attributes! {
                    placeholder="Ask me anything…"
                    :value=$(draft.get())
                    @input=$(|e: Event| draft.set(e.target.value))
                }
            )
            prompt_input_toolbar(
                prompt_input_tools()
                prompt_input_submit(disabled: $(draft.get().trim().is_empty()))
            )
        )
    })
}
