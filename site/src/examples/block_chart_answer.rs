use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use crate::components::{
    chart::{ChartSeries, labels},
    conversation::{conversation, conversation_content},
    line_chart::line_chart,
    message::{MessageRole, message, message_content},
    tool_call::{ToolStatus, tool_call, tool_call_input, tool_call_output},
};

/// What the tool returned: each day's name, short name, high and low in °C.
const FORECAST: [(&str, &str, f64, f64); 7] = [
    ("Monday", "Mon", 21.0, 12.0),
    ("Tuesday", "Tue", 19.0, 11.0),
    ("Wednesday", "Wed", 17.0, 10.0),
    ("Thursday", "Thu", 18.0, 9.0),
    ("Friday", "Fri", 22.0, 12.0),
    ("Saturday", "Sat", 23.0, 14.0),
    ("Sunday", "Sun", 20.0, 13.0),
];

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    // The answer's summary comes from the same data as the chart.
    let warmest = FORECAST
        .iter()
        .max_by(|a, b| a.2.total_cmp(&b.2))
        .expect("the forecast has days");
    let coolest = FORECAST
        .iter()
        .min_by(|a, b| a.3.total_cmp(&b.3))
        .expect("the forecast has days");

    Ok(view! {
        <div class="flex h-full w-full flex-col">
            conversation(
                conversation_content(
                    message(
                        role: MessageRole::User,
                        message_content("What's the weather in Kyoto next week?")
                    )
                    message(
                        role: MessageRole::Assistant,
                        message_content(
                            tool_call(
                                name: "get_forecast",
                                status: ToolStatus::Completed,
                                tool_call_input("{ \"city\": \"Kyoto\", \"days\": 7 }")
                                tool_call_output(
                                    "{ \"high\": [21, 19, 17, 18, 22, 23, 20], \"low\": [12, 11, \
                                     10, 9, 12, 14, 13] }"
                                )
                            )
                            <p>"Here are the highs and lows for the week, in °C:"</p>
                            // The chart comes from the topcoat-graph registry. It is a
                            // component like any other, so it goes where the text goes.
                            <div
                                class="w-full max-w-md rounded-lg border border-border bg-card p-4"
                            >
                                line_chart(
                                    label: "Kyoto forecast for the next seven days",
                                    category_header: "Day",
                                    categories: labels(FORECAST.map(|day| day.1)),
                                    series: vec![
                                        ChartSeries::new("High", FORECAST.map(|day| day.2)),
                                        ChartSeries::new("Low", FORECAST.map(|day| day.3)),
                                    ],
                                    dots: true,
                                    legend: true,
                                    attrs: attributes! { class="aspect-auto h-48" }
                                )
                            </div>
                            <p>
                                (warmest.0)
                                " is the warmest day at "
                                (warmest.2)
                                "°. "
                                (coolest.0)
                                " has the coldest night, down to "
                                (coolest.3)
                                "°."
                            </p>
                        )
                    )
                    message(
                        role: MessageRole::User,
                        message_content("Which day is best for the Fushimi Inari hike?")
                    )
                    message(
                        role: MessageRole::Assistant,
                        message_content(
                            "Wednesday. It's the coolest day of the week, which suits a two-hour \
                             climb. Start early and bring a layer for the shade."
                        )
                    )
                )
            )
        </div>
    })
}
