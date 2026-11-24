use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::code_block::code_block;

const PYTHON: &str = "from pathlib import Path

def word_count(path: str) -> int:
    \"\"\"Counts the words in a text file.\"\"\"
    return len(Path(path).read_text().split())  # whitespace-separated
";

const TSX: &str = "export function Greeting({ name }: { name: string }) {
  const [count, setCount] = useState<number>(0);
  return <button onClick={() => setCount(count + 1)}>Hi {name}, {count}</button>;
}
";

const SHELL: &str =
    "# Build and start the server\ncargo build --release && PORT=8080 ./target/release/garden\n";

const SQL: &str = "SELECT name, COUNT(*) AS orders
FROM customers JOIN orders ON orders.customer_id = customers.id
WHERE orders.created_at > '2026-01-01'
GROUP BY name;
";

const YAML: &str = "name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest  # the default image
";

const DIFF: &str = "@@ -1,3 +1,3 @@
 fn main() {
-    println!(\"Hello\");
+    println!(\"Hello, garden\");
 }
";

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // `language` picks the highlighting. Aliases such as `py`, `ts`, `sh` and `yml`
        // work too.
        <div class="flex w-full flex-col gap-4">
            code_block(language: Some("python"), code: PYTHON)
            code_block(language: Some("tsx"), code: TSX)
            code_block(language: Some("sh"), code: SHELL)
            code_block(language: Some("sql"), code: SQL)
            code_block(language: Some("yaml"), code: YAML)
            code_block(language: Some("diff"), code: DIFF)
        </div>
    })
}
