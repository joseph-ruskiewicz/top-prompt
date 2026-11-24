use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    conversation::{conversation, conversation_content},
    message::{MessageRole, message, message_content},
};

const QUESTION: &str = "How do I keep basil alive indoors?";

const ANSWER: &str = "Basil wants three things indoors: light, warmth and steady water. \
    Give it the sunniest window you have, ideally south-facing, and at least six hours of \
    direct sun; in winter a small grow light helps. Keep it away from cold glass at night. \
    Water when the top centimetre of soil is dry, in the morning, and empty the saucer \
    afterwards so the roots never sit in water. Pinch off the top pair of leaves on each \
    stem every week or two: the plant branches out instead of growing tall, and flowers \
    later. When it does try to flower, pinch the buds off, because flowering makes the \
    leaves bitter. Feed it lightly once a month in spring and summer. Repot into a bigger \
    pot with drainage holes when roots appear at the bottom.";

/// Stands in for the model: waits as long as one chunk of a reply takes to arrive.
#[procedure]
async fn next_chunk(cx: &Cx) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(120)).await;
    Ok(())
}

/// Re-renders as words arrive. In an app, a live region renders the conversation again
/// as the model streams; here a loop in the browser stands in for it.
#[shard]
async fn chat(cx: &Cx) -> Result<impl View> {
    let words: Vec<&str> = ANSWER.split(' ').collect();
    let shown = signal(cx, || words.len());
    let count = shown.get().min(words.len());
    let streaming = count < words.len();
    let reply = words[..count].join(" ");
    let total = words.len();

    Ok(view! {
        <div class="flex w-full flex-col gap-3">
            <div
                class="flex h-64 flex-col overflow-hidden rounded-xl border border-border"
            >
                // `aria-busy` while streaming, so screen readers read the reply once it
                // is complete instead of word by word.
                conversation(
                    attrs: attributes! { aria-busy=(if streaming { "true" } else { "false" }) },
                    conversation_content(
                        message(
                            role: MessageRole::User,
                            message_content("I just bought a basil plant.")
                        )
                        message(
                            message_content(
                                "Nice! Supermarket basil is often several \
                            seedlings in one pot; splitting them into two or three pots helps \
                            them last."
                            )
                        )
                        message(role: MessageRole::User, message_content((QUESTION)))
                        message(message_content((reply)))
                    )
                )
            </div>
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    class="self-start"
                    disabled=(streaming)
                    @click=$(async |_e: Event| {
                        shown.set(0);
                        while shown.get() < total {
                            next_chunk().await;
                            shown.set(shown.get() + 3);
                        }
                        shown.set(total);
                    })
                },
                "Stream the reply again"
            )
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { chat() })
}
