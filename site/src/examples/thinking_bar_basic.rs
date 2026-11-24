use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use crate::components::thinking_bar::thinking_bar;

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    let answered = signal(cx, || false);

    Ok(view! {
        <div class="flex w-full max-w-md flex-col gap-6">
            thinking_bar()
            <div :hidden=$(answered.get())>
                thinking_bar(
                    text: "Comparing three train routes",
                    stop_label: Some("Answer now"),
                    // Tell your model to stop thinking here, for example with a procedure.
                    stop_attrs: attributes! { @click=$(|_e: Event| answered.set(true)) }
                )
            </div>
            <p class="text-sm" :hidden=$(!answered.get())>
                "The 9:05 is fastest: 2 hours 49 minutes, no changes."
            </p>
        </div>
    })
}
