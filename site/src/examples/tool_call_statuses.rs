use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::tool_call::{ToolStatus, tool_call, tool_call_input, tool_call_output};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex w-full flex-col gap-3">
            tool_call(
                name: "convert_currency",
                status: ToolStatus::Pending,
                tool_call_input(
                    "{ \"amount\": 120, \"from\": \"EUR\", \"to\": \"JPY\" }"
                )
            )
            tool_call(
                name: "search_flights",
                status: ToolStatus::Running,
                tool_call_input("{ \"from\": \"OPO\", \"to\": \"HND\" }")
            )
            tool_call(
                name: "get_forecast",
                status: ToolStatus::Completed,
                tool_call_input("{ \"city\": \"Kyoto\", \"days\": 3 }")
                tool_call_output("[\"sun 21°\", \"cloud 19°\", \"rain 17°\"]")
            )
            tool_call(
                name: "book_table",
                status: ToolStatus::Error,
                tool_call_input("{ \"restaurant\": \"Kikunoi\", \"people\": 4 }")
                tool_call_output(error: true, "No tables free on that date.")
            )
        </div>
    })
}
