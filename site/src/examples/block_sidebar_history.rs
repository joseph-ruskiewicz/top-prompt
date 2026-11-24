use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    conversation::{conversation, conversation_content},
    message::{MessageRole, message, message_content},
    prompt_input::{
        prompt_input, prompt_input_submit, prompt_input_textarea, prompt_input_toolbar,
        prompt_input_tools,
    },
};

/// Past chats, grouped by when they happened.
const HISTORY: [(&str, &[&str]); 3] = [
    ("Today", &["Basil care", "Bakery cat names"]),
    ("Yesterday", &["Kyoto itinerary", "Rent reminder email"]),
    (
        "Last week",
        &[
            "Compound interest",
            "Sourdough schedule",
            "Hiking packing list",
        ],
    ),
];

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex h-full w-full">
            <nav
                aria-label="Chat history"
                class="hidden w-56 shrink-0 flex-col gap-4 border-r border-border p-3 sm:flex"
            >
                button(
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    attrs: attributes! { type="button" },
                    icon(data: iconify_icon!("lucide:square-pen"))
                    "New chat"
                )
                <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto">
                    for (when, chats) in HISTORY {
                        <div class="flex flex-col gap-0.5">
                            <span
                                class="px-2 pb-1 text-xs font-medium text-muted-foreground"
                            >
                                (when)
                            </span>
                            for title in chats.iter() {
                                <a
                                    href="#"
                                    aria-current=((*title == "Basil care").then_some("page"))
                                    class="truncate rounded-md px-2 py-1.5 text-sm text-foreground/80 outline-none hover:bg-foreground/5 focus-visible:ring-2 focus-visible:ring-ring aria-[current=page]:bg-foreground/5 aria-[current=page]:font-medium aria-[current=page]:text-foreground"
                                >
                                    (title)
                                </a>
                            }
                        </div>
                    }
                </div>
            </nav>
            <div class="flex min-w-0 flex-1 flex-col">
                conversation(
                    conversation_content(
                        message(
                            role: MessageRole::User,
                            message_content(
                                "My basil keeps wilting. What am I doing wrong?"
                            )
                        )
                        message(
                            message_content(
                                "Usually it is water or light: moist, not soggy, soil and six \
                                hours of sun."
                            )
                        )
                    )
                )
                <div class="w-full shrink-0 px-4 pb-4">
                    prompt_input(
                        prompt_input_textarea(
                            attrs: attributes! { placeholder="Reply…" }
                        )
                        prompt_input_toolbar(
                            prompt_input_tools()
                            prompt_input_submit()
                        )
                    )
                </div>
            </div>
        </div>
    })
}
