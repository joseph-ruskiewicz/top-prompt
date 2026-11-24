use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use crate::components::code_block::code_block;

const LOG: &str = "$ cargo build --release
   Compiling serde v1.0.219
   Compiling tokio v1.47.1
   Compiling hyper v1.7.0
   Compiling axum v0.8.4
   Compiling topcoat v0.12.0
   Compiling garden-planner v0.3.1 (/home/ana/garden-planner)
    Finished `release` profile [optimized] target(s) in 1m 12s
$ ./target/release/garden-planner --port 8080
listening on http://0.0.0.0:8080
";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // No `language`: the header reads "Code". Classes in `attrs` cap the height, and
        // the block scrolls in both directions instead of wrapping.
        code_block(
            code: LOG,
            attrs: attributes! { class="w-full [&_pre]:max-h-48 [&_pre]:overflow-y-auto" }
        )
    })
}
