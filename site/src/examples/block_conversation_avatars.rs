use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::{
    conversation::{conversation, conversation_content},
    message::{MessageRole, message, message_avatar, message_content},
};

const MESSAGES: [(MessageRole, &str); 6] = [
    (
        MessageRole::User,
        "My basil keeps wilting. What am I doing wrong?",
    ),
    (
        MessageRole::Assistant,
        "Usually it is water or light. Basil likes moist, not soggy, soil and at least six \
         hours of sun. Is the pot draining?",
    ),
    (MessageRole::User, "It sits in a saucer of water most days."),
    (
        MessageRole::Assistant,
        "That is likely it: the roots stay wet and rot. Empty the saucer after watering and \
         let the top centimeter dry out between waterings.",
    ),
    (MessageRole::User, "Should I cut it back too?"),
    (
        MessageRole::Assistant,
        "Yes. Pinch off the top pair of leaves on each stem; it grows bushier and lasts longer.",
    ),
];

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex h-full w-full flex-col">
            conversation(
                conversation_content(
                    for (role, text) in MESSAGES {
                        message(
                            role: role,
                            message_avatar(
                                fallback: if role == MessageRole::User {
                                    "MS"
                                } else {
                                    "AI"
                                }
                            )
                            message_content((text))
                        )
                    }
                )
            )
        </div>
    })
}
