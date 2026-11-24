use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::markdown::markdown;

const ANSWER: &str = "### Sourdough schedule

Feed the starter **the night before**, then:

1. Mix flour and water, rest *45 minutes*
2. Add starter and salt
3. Fold every 30 minutes, four times

| Step | Time |
| --- | --- |
| Bulk rise | 4–6 h |
| Cold proof | overnight |

```text
oven: 250°C with lid, then 230°C without
```

- [x] Starter fed
- [ ] Banneton floured

See the [bread guide](https://example.com/bread). <b>Raw HTML</b> stays text.";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { <div class="w-full">markdown(source: ANSWER)</div> })
}
