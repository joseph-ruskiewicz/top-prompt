use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::tool_call::{
    ToolStatus, tool_call_header, tool_call_input, tool_call_output,
};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // `tool_call` starts closed. To show a call open, for example one that needs the
        // reader's attention, build the card around `tool_call_header` yourself.
        <details
            open=""
            class="group/tool w-full rounded-lg border border-border bg-card"
        >
            tool_call_header(name: "send_email", status: ToolStatus::Error)
            <div class="flex flex-col gap-3 border-t border-border p-3">
                tool_call_input(
                    "{ \"to\": \"ana@example.com\", \"subject\": \"Lisbon plans\" }"
                )
                tool_call_output(error: true, "The mail server refused the connection.")
            </div>
        </details>
    })
}
