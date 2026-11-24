use topcoat::{
    Result,
    context::Cx,
    icon::iconify::iconify_icon,
    view::{View, component, view},
};

use crate::components::steps::{steps, steps_content, steps_item, steps_trigger};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="w-full max-w-md">
            steps(
                default_open: true,
                steps_trigger(
                    leading_icon: Some(iconify_icon!("lucide:search")),
                    "Looked up train times"
                )
                steps_content(
                    steps_item("Searched for trains from Porto to Lisbon on Saturday")
                    steps_item("Read the timetable on 2 sites")
                    steps_item("Found 6 departures between 7:00 and 19:00")
                )
            )
        </div>
    })
}
