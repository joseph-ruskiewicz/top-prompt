use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::steps::{steps_bar, steps_item};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // Without the collapsible: the bar and items on their own, always visible.
        <div class="flex w-full max-w-md gap-3">
            steps_bar()
            <div class="flex flex-col gap-2">
                steps_item("Read 3 reviews of the hotel")
                steps_item("Checked the distance to the station: 400 m")
                steps_item("Compared the price with two hotels nearby")
            </div>
        </div>
    })
}
