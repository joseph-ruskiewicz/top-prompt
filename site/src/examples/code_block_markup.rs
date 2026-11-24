use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::code_block::code_block;

const OUTPUT: &str = "   Compiling garden v0.3.1
error[E0308]: mismatched types
 --> src/main.rs:4:18
warning: unused variable: `soil`
";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // Child content replaces the text inside `<code>`, for markup of your own or
        // another highlighter. It must show the same text as `code`, which is what the
        // copy button copies.
        <div class="w-full">
            code_block(
                language: Some("output"),
                code: OUTPUT,
                "   Compiling garden v0.3.1\n"
                <span class="font-medium text-destructive">
                    "error[E0308]: mismatched types"
                </span>
                "\n --> src/main.rs:4:18\n"
                <span class="text-muted-foreground">
                    "warning: unused variable: `soil`"
                </span>
                "\n"
            )
        </div>
    })
}
