use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    prompt_input::{
        prompt_input, prompt_input_submit, prompt_input_textarea, prompt_input_toolbar,
    },
    suggestion::{suggestion, suggestions},
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let draft = signal(cx, String::new);

    Ok(view! {
        <div class="flex w-full flex-col gap-3">
            suggestions(
                label: "Starter prompts",
                // Each button carries its prompt as its value; `label` is the shorter text
                // it shows.
                #[key(prompt)]
                for (prompt, label) in [
                    (
                        "Plan a three-day trip to Kyoto in April, with one rest day",
                        "Kyoto trip",
                    ),
                    (
                        "Suggest a weeknight dinner with chickpeas and spinach",
                        "Quick dinner",
                    ),
                    (
                        "Explain compound interest with a worked example",
                        "Compound interest",
                    ),
                ] {
                    suggestion(
                        prompt: prompt,
                        label: label,
                        attrs: attributes! { @click=$(|e: Event| draft.set(e.current_target.value)) }
                    )
                }
            )
            prompt_input(
                attrs: attributes! { @submit=$(|e: Event| e.prevent_default()) },
                prompt_input_textarea(
                    attrs: attributes! {
                        placeholder="Pick a suggestion or type your own…"
                        :value=$(draft.get())
                        @input=$(|e: Event| draft.set(e.target.value))
                    }
                )
                prompt_input_toolbar(
                    <span></span>
                    prompt_input_submit(disabled: $(draft.get().trim().is_empty()))
                )
            )
        </div>
    })
}
