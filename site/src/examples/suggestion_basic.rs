use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use crate::components::suggestion::{suggestion, suggestions};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        suggestions(
            attrs: attributes! { class="justify-center" },
            suggestion(prompt: "Explain closures with an example")
            suggestion(prompt: "Write a regex for UK postcodes")
            suggestion(
                prompt: "What changed in the 2024 edition of Rust?",
                label: "Rust 2024 changes"
            )
        )
    })
}
