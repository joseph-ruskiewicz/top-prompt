use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::code_block::code_block;

const QUERY: &str = "SELECT * FROM plants WHERE name = 'basil';\n";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // `highlight: false` shows the code as plain text, whatever its language.
        <div class="w-full">
            code_block(language: Some("sql"), code: QUERY, highlight: false)
        </div>
    })
}
