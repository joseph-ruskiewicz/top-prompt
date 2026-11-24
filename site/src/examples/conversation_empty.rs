use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    view::{View, attributes, component, view},
};

use crate::components::conversation::{
    conversation, conversation_content, conversation_empty_state,
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div
            class="flex h-64 w-full flex-col overflow-hidden rounded-xl border border-border"
        >
            conversation(
                conversation_content(
                    conversation_empty_state(
                        title: "Ask about your garden",
                        description: "Plants, pests, watering, and what to sow this month.",
                        icon(
                            data: iconify_icon!("lucide:sprout"),
                            attrs: attributes! { class="order-first size-8 text-muted-foreground" }
                        )
                    )
                )
            )
        </div>
    })
}
