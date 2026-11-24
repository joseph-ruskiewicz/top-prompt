use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::sources::{source, sources_content, sources_trigger};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // `sources` starts closed. For a list that starts open, build the `<details>`
        // yourself around the trigger and content.
        <details open="" class="group/sources w-full">
            sources_trigger(count: 2)
            sources_content(
                source(href: "https://www.rust-lang.org/learn", title: "Learn Rust")
                source(
                    href: "https://doc.rust-lang.org/std/",
                    title: "The standard library"
                )
            )
        </details>
    })
}
