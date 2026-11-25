# OpenAI support for the real-model demo

## Why

The live demo at `/full/claude` only takes an Anthropic key. Adding OpenAI:

1. shows the topcoat-prompt components don't depend on one provider, and
2. lets visitors who have an OpenAI key but no Claude key try the demo.

## Goals

- A visitor picks **Anthropic** or **OpenAI**, pastes that provider's key, picks
  a model, and gets the same experience: streamed markdown text, reasoning in
  `reasoning` when the model returns it, the two existing tools in `tool_call`
  (Pending → Running → Completed), Stop, Regenerate with the branch picker,
  Copy, error notices with "Change key".
- Everything below the key form is identical UI for both providers. That sameness
  is the point of the feature.

## Non-goals

- Other providers (Gemini, OpenRouter, Groq and so on). The user chose only
  Anthropic and OpenAI. The design shouldn't block adding them later, but
  nothing is built for them now.
- Carrying a conversation across providers.
- Any change to the registry crate (`crates/topcoat-prompt`). This is site-only.

## Anti-goals (failure even if it works)

- Any weakening of the key handling from PR #10: the key never in a cookie, URL,
  HTML, signal, log, storage or error text; memory only; dropped on Clear key,
  30 minutes idle, eviction or restart.
- Provider-specific UI branching that makes the components look tailored to one
  provider.
- Letting browser input choose the endpoint host.

## Approaches

- **A. One page with a provider choice (chosen).** One page, a provider toggle
  on the key form, a per-provider adapter behind a shared update type. Best fits
  "the components don't depend on one provider".
- B. A separate `/full/openai` page sharing internals. Same adapter work plus a
  second page, and two pages suggest per-provider builds. Rejected.
- C. A gateway such as OpenRouter. Visitors would need a third account and a
  third party would see their keys. Rejected.

## Decided

- Providers: Anthropic and OpenAI only.
- One page. It moves from `/full/claude` to `/full/live`. `/full/claude`
  redirects there so existing links keep working. Sidebar, `/blocks` card, the
  fake demo's empty state, catalog text and `llms.txt` are updated.
- The key form gets an "Anthropic | OpenAI" choice. Key validation is per
  provider (`sk-ant-` for Anthropic, OpenAI's documented key prefixes for
  OpenAI). The model picker lists only the chosen provider's models.
- A visitor holds one key for one provider at a time. Switching provider clears
  the key and the conversation.
- Conversation history is stored in a provider-neutral form in `chat.rs`, and
  each adapter converts it to its provider's request format. Each adapter turns
  its provider's stream into the existing `Update` events, so `chat.rs` and the
  view don't know which provider is running.
- The OpenAI host is a fixed constant, like `PRODUCTION_URL` for Anthropic.
  Release builds can only reach those two hosts. Debug and test builds may point
  either adapter at the fake server.
- Same limits as now (model allow-list, 4,000 chars per message, 20 messages,
  max output tokens, 5 tool rounds, 20 requests per 10 minutes per session and
  40 per address), applied the same way to both providers.
- The privacy and billing wording on the page is reworded to cover both
  providers, with a link to each provider's key page, recommending a restricted
  low-limit key.
- `fake.rs` gains an OpenAI mode, and the existing test matrix (text, reasoning,
  tools, errors 401/429/5xx, stop mid-stream and mid-tool, regenerate, two
  visitors kept apart, key absent from pages/headers/Debug) runs against both.

## Left to the builder

- Which OpenAI API to use. Prefer the one OpenAI's current docs recommend for new
  work that streams reasoning summaries and supports function tools; confirm
  from the official docs, not memory.
- Which two or three OpenAI models to offer, and the default, taken from
  OpenAI's official model docs at build time.
- How OpenAI's error responses map onto the existing `ErrorKind`s.
- Module naming (for example, renaming `site/src/claude/` to `site/src/live/`
  with `anthropic.rs` and `openai.rs` adapters).
- Whether to hide the reasoning block or show nothing when a model returns no
  reasoning. Don't show an empty one.

## Verification

Workspace build, clippy `-D warnings`, `cargo fmt --check`, `topcoat fmt --check`,
`cargo test --workspace`, and a browser pass against the fake server in both
provider modes, light and dark, covering everything in Goals plus Clear key,
provider switching, the `/full/claude` redirect, and the key absent from the
DOM, storage, URLs and server output. No real API calls. The user tests with
real keys after deploy.
