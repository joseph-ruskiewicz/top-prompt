use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::sources::source_chip;

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex flex-wrap gap-2">
            source_chip(
                href: "https://www.rust-lang.org/learn",
                title: "Learn Rust",
                description: "Books, courses and references for learning Rust.",
                show_favicon: true
            )
            source_chip(
                href: "https://doc.rust-lang.org/book/",
                label: "The Book",
                title: "The Rust Programming Language",
                description: "The official introduction to Rust, from the basics up."
            )
            source_chip(href: "https://crates.io/", title: "crates.io")
        </div>
    })
}
