use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::reasoning::{reasoning, reasoning_content};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex w-full flex-col gap-6">
            reasoning(
                streaming: true,
                reasoning_content(
                    "The reader asked for a vegetarian lasagne for eight. I should scale \
                    the recipe by two and swap the beef for lentils…"
                )
            )
            reasoning(
                duration_secs: Some(6),
                reasoning_content(
                    "Doubling everything gives two trays. Lentils need less salt than \
                    beef, so I will reduce it and mention it."
                )
            )
        </div>
    })
}
