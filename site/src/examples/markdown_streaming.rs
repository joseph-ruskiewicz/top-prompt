use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    markdown::markdown,
};

const ANSWER: &str = "Read the file line by line, so a large file never sits in memory:

```python
from pathlib import Path

def longest_line(path: str) -> str:
    \"\"\"Returns the longest line, without its line break.\"\"\"
    with Path(path).open(encoding=\"utf-8\") as file:
        return max((line.rstrip(\"\\n\") for line in file), key=len, default=\"\")
```

`max` reads the lines lazily. See the \
[`pathlib` docs](https://docs.python.org/3/library/pathlib.html) for more.";

/// Stands in for the model: waits as long as one chunk takes to arrive.
#[procedure]
async fn next_chunk(cx: &Cx) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(60)).await;
    Ok(())
}

/// Renders whatever has arrived so far. Half a code fence or half a link renders as far
/// as it goes, so there is nothing to buffer or repair while streaming.
#[shard]
async fn answer(cx: &Cx) -> Result<impl View> {
    let total = ANSWER.chars().count();
    let shown = signal(cx, || total);
    let partial: String = ANSWER.chars().take(shown.get()).collect();

    Ok(view! {
        <div class="flex w-full flex-col gap-3">
            <div class="min-h-72">markdown(source: &partial)</div>
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    class="self-start"
                    disabled=(shown.get() < total)
                    @click=$(async |_e: Event| {
                        shown.set(0);
                        while shown.get() < total {
                            next_chunk().await;
                            shown.set(shown.get() + 8);
                        }
                        shown.set(total);
                    })
                },
                "Stream it again"
            )
        </div>
    })
}

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { answer() })
}
