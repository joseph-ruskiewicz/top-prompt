use topcoat::{
    Result,
    context::Cx,
    view::{Unescaped, View, component, view},
};

use crate::{components::code_block::code_block, highlight};

const CODE: &str = "let total: u32 = items.iter().map(|item| item.price).sum();
println!(\"total: {total}\");
";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    // Any highlighter works, as long as its output is the same text wrapped in
    // elements. This site uses a small one of its own.
    let markup = Unescaped::new_unchecked(highlight::rust(CODE));

    Ok(view! {
        <div class="w-full">
            code_block(language: Some("rust"), code: CODE, (markup))
        </div>
    })
}
