use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::markdown::markdown;

const ANSWER: &str = "Install the client, then send a request:

```sh
npm install @garden/client
```

```ts
import { Garden } from \"@garden/client\";

const garden = new Garden({ token: process.env.GARDEN_TOKEN });
const plants = await garden.plants.list({ limit: 10 });
```

The response is JSON:

```json
{ \"plants\": [{ \"name\": \"basil\", \"water_every_days\": 2 }], \"next\": null }
```

A fence with a language the highlighter does not know is shown as plain text:

```text
basil    every 2 days
cactus   every 21 days
```
";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! { <div class="w-full">markdown(source: ANSWER)</div> })
}
