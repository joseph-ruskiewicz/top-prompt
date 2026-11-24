use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::{
    conversation::{conversation, conversation_content},
    message::{MessageRole, message, message_content},
};

const MESSAGES: [(MessageRole, &str); 6] = [
    (
        MessageRole::User,
        "What should I pack for two days of hiking?",
    ),
    (
        MessageRole::Assistant,
        "Layers, a rain shell, two liters of water a day, snacks, a map, a headlamp, and a \
         small first-aid kit.",
    ),
    (MessageRole::User, "Is a tent worth it if there are huts?"),
    (
        MessageRole::Assistant,
        "If the huts take reservations and you have one, leave the tent. Otherwise a light \
         one is cheap insurance.",
    ),
    (MessageRole::User, "And boots or trail runners?"),
    (
        MessageRole::Assistant,
        "Trail runners if the paths are dry and you are used to them; boots for rocky ground \
         or a heavy pack.",
    ),
];

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div
            class="flex h-80 w-full flex-col overflow-hidden rounded-xl border border-border"
        >
            conversation(
                conversation_content(
                    for (role, text) in MESSAGES {
                        message(role: role, message_content((text)))
                    }
                )
            )
        </div>
    })
}
