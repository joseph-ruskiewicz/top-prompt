use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::code_block::code_block;

const CODE: &str = "fn fahrenheit(celsius: f64) -> f64 {
    celsius * 9.0 / 5.0 + 32.0
}
";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { <div class="w-full">code_block(language: Some("rust"), code: CODE)</div> })
}
