use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use crate::components::markdown::markdown;

/// Model output that tries a few tricks.
const OUTPUT: &str = "Here is the summary you asked for.

<script>alert('hi')</script> <img src=x onerror=alert(1)>

- [A normal link](https://example.com/report) opens in a new tab.
- [A script link](javascript:alert(1)) is shown as plain text.
- ![A tracking pixel](ftp://example.com/p.gif) is shown as its alt text.
- Contact [the author](mailto:ana@example.com).";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // Safe as it is: HTML stays text, and only http, https and mailto links survive.
        // `attrs` add classes to the wrapper, here a smaller text size.
        markdown(source: OUTPUT, attrs: attributes! { class="w-full text-sm" })
    })
}
