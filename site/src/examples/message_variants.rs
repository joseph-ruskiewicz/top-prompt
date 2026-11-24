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
            // Auto, the default: a bubble for the user, flat for everyone else.
            message(
                role: MessageRole::User,
                message_content(
                    variant: MessageContentVariant::Auto,
                    "Auto, from the user"
                )
            )
            message(
                role: MessageRole::Assistant,
                message_content(
                    variant: MessageContentVariant::Auto,
                    "Auto, from the assistant"
                )
            )
            // Or choose the look whatever the role.
            message(
                role: MessageRole::User,
                message_avatar(fallback: "RN")
                message_content(
                    variant: MessageContentVariant::Flat,
                    "Flat, from the user"
                )
            )
            message(
                role: MessageRole::Assistant,
                message_avatar(fallback: "AI")
                message_content(
                    variant: MessageContentVariant::Bubble,
                    "A bubble, from the assistant"
                )
            )
        </div>
    })
}
