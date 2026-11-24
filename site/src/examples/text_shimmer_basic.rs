use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use crate::components::text_shimmer::text_shimmer;

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex flex-col items-center gap-4">
            text_shimmer(attrs: attributes! { class="text-sm" }, "Reading 12 pages…")
            text_shimmer(
                duration_ms: 3000,
                spread: 80,
                attrs: attributes! { class="text-2xl font-medium" },
                "Composing an answer"
            )
        </div>
    })
}
