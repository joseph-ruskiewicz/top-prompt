use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::reasoning::{reasoning_content, reasoning_trigger};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // `reasoning` closes once the reply is done. To keep finished reasoning open, put
        // the trigger and content in a `<details>` of your own.
        <details open="" class="group/reasoning w-full">
            reasoning_trigger(duration_secs: Some(12))
            reasoning_content(
                "Three nights in Lisbon and two in Porto fits the dates. The train between \
                them is faster than flying once you count the airports."
            )
        </details>
    })
}
