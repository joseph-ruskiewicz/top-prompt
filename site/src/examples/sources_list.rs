use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::sources::{source, sources, sources_content};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="w-full">
            sources(
                count: 3,
                sources_content(
                    source(href: "https://www.rust-lang.org/learn", title: "Learn Rust")
                    source(
                        href: "https://doc.rust-lang.org/book/",
                        title: "The Rust Book"
                    )
                    source(href: "https://doc.rust-lang.org/std/")
                )
            )
        </div>
    })
}
