use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::message::{
    MessageContentVariant, MessageRole, message, message_avatar, message_content,
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex w-full flex-col gap-6">
            message(
                role: MessageRole::System,
                message_content("Model switched to a faster one")
            )
            message(
                role: MessageRole::User,
                message_avatar(fallback: "RN")
                message_content("Translate \"good morning\" into Portuguese.")
            )
            message(
                message_avatar(fallback: "AI")
                message_content("\"Bom dia.\" In the afternoon, say \"boa tarde\".")
            )
            message(
                message_content(
                    variant: MessageContentVariant::Bubble,
                    "Assistant replies can use the bubble too."
                )
            )
        </div>
    })
}
