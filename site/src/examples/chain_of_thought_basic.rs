use topcoat::{
    Result,
    context::Cx,
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
                    default_open: true,
                    chain_of_thought_trigger("Understand the question")
                    chain_of_thought_content(
                        chain_of_thought_item(
                            "The reader wants to split a bill of 84.60 between three people."
                        )
                        chain_of_thought_item(
                            "They mentioned one person skipped dessert."
                        )
                    )
                )
                chain_of_thought_step(
                    chain_of_thought_trigger("Work out the shares")
                    chain_of_thought_content(
                        chain_of_thought_item(
                            "Dessert was 12.00, shared by two people."
                        )
                        chain_of_thought_item(
                            "The rest, 72.60, splits three ways: 24.20 each."
                        )
                    )
                )
                chain_of_thought_step(
                    chain_of_thought_trigger("Check the total")
                    chain_of_thought_content(
                        chain_of_thought_item(
                            "30.20 + 30.20 + 24.20 = 84.60. It adds up."
                        )
                    )
                )
            )
        </div>
    })
}
