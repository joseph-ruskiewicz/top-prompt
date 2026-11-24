use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::{
    conversation::{conversation, conversation_content},
    markdown::markdown,
    message::{MessageRole, message, message_content},
};

/// A long exchange, so the log scrolls.
const STEPS: [(&str, &str); 5] = [
    (
        "How do I start a vegetable bed?",
        "Pick the sunniest spot you have: six hours at least.",
    ),
    (
        "Raised or in the ground?",
        "Raised beds drain better and warm up sooner in spring.",
    ),
    (
        "What do I fill it with?",
        "**Two parts** topsoil to one part compost, mixed well.",
    ),
    (
        "What should I plant first?",
        "Lettuce, radishes and peas: quick and forgiving.",
    ),
    (
        "How often do I water?",
        "When the top 2–3 cm are dry. Mornings are best:\n\n- less evaporation\n- leaves \
         dry before night",
    ),
];

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex h-full w-full flex-col">
            // The log starts at the bottom. Scroll up to show the button back down.
            conversation(
                conversation_content(
                    for (question, answer) in STEPS {
                        message(role: MessageRole::User, message_content((question)))
                        message(message_content(markdown(source: answer)))
                    }
                )
            )
        </div>
    })
}
