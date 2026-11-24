use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::sources::inline_citation;

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <p class="max-w-md text-sm leading-relaxed">
            "Rust guarantees memory safety without a garbage collector"
            inline_citation(
                label: "1",
                href: "https://doc.rust-lang.org/book/",
                title: "The Rust Book"
            )
            ", and its package manager builds and fetches dependencies"
            inline_citation(
                label: "2",
                href: "https://doc.rust-lang.org/cargo/",
                title: "The Cargo Book"
            )
            "."
        </p>
    })
}
