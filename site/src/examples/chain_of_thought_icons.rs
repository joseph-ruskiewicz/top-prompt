use topcoat::{
    Result,
    context::Cx,
    icon::iconify::iconify_icon,
    view::{View, component, view},
};

use crate::components::chain_of_thought::{
    chain_of_thought, chain_of_thought_content, chain_of_thought_item, chain_of_thought_step,
    chain_of_thought_trigger,
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="w-full max-w-md">
            chain_of_thought(
                chain_of_thought_step(
                    chain_of_thought_trigger(
                        leading_icon: Some(iconify_icon!("lucide:search")),
                        "Searched the recipe archive"
                    )
                    chain_of_thought_content(
                        chain_of_thought_item(
                            "Found 14 lentil recipes, 5 of them for eight."
                        )
                    )
                )
                chain_of_thought_step(
                    chain_of_thought_trigger(
                        leading_icon: Some(iconify_icon!("lucide:scale")),
                        "Scaled the quantities"
                    )
                    chain_of_thought_content(
                        chain_of_thought_item("Doubled everything except the salt.")
                        chain_of_thought_item("Two 30 × 20 cm trays instead of one.")
                    )
                )
                chain_of_thought_step(
                    default_open: true,
                    chain_of_thought_trigger(
                        leading_icon: Some(iconify_icon!("lucide:circle-check")),
                        "Checked the timing"
                    )
                    chain_of_thought_content(
                        chain_of_thought_item(
                            "Both trays fit in one oven at 180°C for 45 minutes."
                        )
                    )
                )
            )
        </div>
    })
}
