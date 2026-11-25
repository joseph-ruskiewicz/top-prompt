# OpenAI Provider for the Live Demo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a visitor to the real-model demo choose Anthropic or OpenAI, paste that provider's key, and get the same chat experience on either.

**Architecture:** The demo module (`site/src/claude/`) becomes `site/src/live/`. Everything provider-specific (request body, auth headers, stream parsing, history encoding, error parsing) moves behind a `Provider` enum with one adapter module per provider. `chat.rs` and `view.rs` work only with provider-neutral types (`Update`, `Finished`, `ToolCall`, `Outcome`, `ApiError`). Conversation history stays as opaque per-provider JSON items, because switching provider clears the conversation, so history never crosses providers.

**Tech Stack:** Rust, Topcoat 0.12.1, reqwest 0.13 (rustls/ring), serde_json, the existing SSE parser and fake API.

**Spec:** `docs/superpowers/specs/2026-11-24-openai-provider-design.md`

**Refinement of the spec (flag in the PR):** the spec says history is stored "in a provider-neutral form". This plan keeps each provider's own JSON items in `Reply.api` instead, opaque to `chat.rs`. Since a conversation never crosses providers, a neutral format would add a conversion layer with no user-visible effect. `chat.rs` still never knows which provider is running.

## Global Constraints

- Providers: Anthropic and OpenAI only. Registry crate `crates/topcoat-prompt` is not touched.
- Page moves from `/full/claude` to `/full/live`. `/full/claude` redirects (308) to `/full/live`.
- One key for one provider per visitor. Switching provider clears the key and the conversation and stops any running reply.
- Key handling stays as in PR #10: never in a cookie, URL, HTML, signal, log, storage or error text. `ApiKey` prints `[redacted]`, is not `Clone`/`Serialize`, zeroed on drop, header marked sensitive. Dropped on Clear key, 30 min idle, eviction (500 cap) or restart.
- Release builds reach only `https://api.anthropic.com` and `https://api.openai.com`. Browser input never chooses a host. Debug/test builds may point either provider at the fake.
- Limits unchanged and shared by both providers: model allow-list, 4,000 bytes per message, 20 turns, `MAX_TOKENS` 8,192 output, 5 tool rounds, 20 requests per 10 min per visitor, 40 per address.
- No real API calls anywhere in the build or tests.
- All checks green: `cargo fmt --check`, `rustfmt --check` on registry files (unchanged), `topcoat fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Locally on rustc 1.100, wrap cargo commands in `.github/scripts/cargo-retry.sh` (Topcoat cache bug).
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. A visitor pastes an Anthropic key with OpenAI selected (or the reverse): the key form says which provider the key belongs to, and no request is made.
2. A visitor switches provider while a reply is streaming: the stream stops, key and conversation clear, and no late update from the old task appears.
3. An OpenAI model streams no reasoning summary: no empty reasoning block is shown.
4. OpenAI returns 429 `insufficient_quota` (no credit): the notice says it's a billing problem, not "slow down and retry".
5. An OpenAI error body echoes part of the key: the notice text has it scrubbed, as Anthropic errors already do.

Each has a named test in the task that owns the code.

---

### Task 1: Rename the module and the page

Pure move plus redirect. No behavior change.

**Files:**
- Move: `site/src/claude/` → `site/src/live/` (`git mv`)
- Modify: `site/src/main.rs` (`mod live;`, imports, env var names below), `site/src/shell.rs:197`, `site/src/catalog.rs:780`, `site/src/examples/chat_app.rs:116`, `README.md` and any doc mentioning `/full/claude` or `DEMO_FAKE_ANTHROPIC`
- Modify: `site/src/live/view.rs` (route path)
- Test: `site/src/live/tests.rs`

**Interfaces:**
- Produces: module path `crate::live::{api, chat, cookie, fake, key, sse, tools, view}`; route `/full/live`; debug env var `DEMO_FAKE_API=1` (replaces `DEMO_FAKE_ANTHROPIC`) and `DEMO_ANTHROPIC_BASE_URL` (unchanged).

- [ ] **Step 1: Write the failing test** `old_address_redirects_to_live` in `tests.rs`: `GET /full/claude` returns status 308 with `location: /full/live`; `GET /full/live` returns 200.
- [ ] **Step 2: Run** `.github/scripts/cargo-retry.sh cargo test -p site old_address_redirects` → FAIL (404 or no `/full/live`).
- [ ] **Step 3: `git mv site/src/claude site/src/live`, update every path and link, move the page route to `/full/live`, add a `/full/claude` route that answers 308 to `/full/live`.** Sidebar label stays "Real model demo" for now (Task 5 rewords).
- [ ] **Step 4: Run** `.github/scripts/cargo-retry.sh cargo test --workspace` → all pass, including the existing 73 site tests.
- [ ] **Step 5: Commit** `git commit -m "Move the real-model demo to /full/live"`.

---

### Task 2: Providers, models and keys

**Files:**
- Create: `site/src/live/provider.rs`
- Modify: `site/src/live/api.rs` (move `Model` out), `site/src/live/key.rs`, `site/src/live/chat.rs` (visitor state), `site/src/live/mod.rs`
- Test: `site/src/live/key.rs` (unit tests), `site/src/live/tests.rs`

**Interfaces:**
- Produces, in `provider.rs`:
  - `pub enum Provider { #[default] Anthropic, OpenAi }` with `ALL: [Self; 2]`, `id(self) -> &'static str` (`"anthropic"`, `"openai"`), `label(self) -> &'static str` (`"Anthropic"`, `"OpenAI"`), `from_id(&str) -> Option<Self>`, `keys_url(self) -> &'static str` (`https://platform.claude.com/settings/keys`, OpenAI's API keys page from its docs), `default_model(self) -> Model`.
  - `pub enum Model` covering both providers' models, with `ALL`, `id`, `label`, `from_id`, and `provider(self) -> Provider`, plus `for_provider(p: Provider) -> impl Iterator<Item = Model>`. Anthropic models unchanged (Opus/Sonnet/Haiku 5.5, Sonnet default). OpenAI: two or three current models with tool calling and reasoning summaries, IDs and labels copied from OpenAI's official models page (https://developers.openai.com/api/docs/models), default the mid-tier general model. Record the page and date in a doc comment.
- Produces, in `key.rs`: `ApiKey::parse(provider: Provider, raw: &str) -> Result<ApiKey, KeyError>`. Anthropic: prefix `sk-ant-` as now. OpenAI: prefix `sk-`, and not `sk-ant-`. Same length/charset shape check. New variant `KeyError::WrongProvider(Provider)` with message `"That looks like an {label} key. Switch the provider above, or paste an {other label} key."`. `Prefix` message names the selected provider's prefix.
- Produces, in `chat.rs`: `State.provider: Provider`; `Snapshot.provider: Provider`; `Visitor::set_key(&self, raw: &str) -> Result<(), &'static str>` parses for the current provider; `Visitor::set_provider(&self, p: Provider) -> bool` (true if changed): stops any generation (same as `stop`, so a late task update is ignored by the generation-id check), drops the key, clears turns, sets model to `p.default_model()`. `Visitor::set_model` rejects a model whose `provider()` differs from the visitor's.

- [ ] **Step 1: Write failing unit tests in `key.rs`:**
  - `openai_keys_parse`: `ApiKey::parse(Provider::OpenAi, "sk-proj-abcdefghijklmnopqrstuvwx")` is Ok.
  - `swapped_keys_name_their_provider`: `parse(OpenAi, "sk-ant-api03-…")` → `Err(KeyError::WrongProvider(Provider::Anthropic))`; `parse(Anthropic, "sk-proj-…")` → `Err(KeyError::WrongProvider(Provider::OpenAi))`. (Review Focus 1)
  - existing Anthropic parse tests updated to pass `Provider::Anthropic`.
- [ ] **Step 2: Write failing tests in `tests.rs`:**
  - `switching_provider_clears_key_and_conversation`: visitor with an Anthropic key sends `[text] hi`, settles; `set_provider(OpenAi)` → snapshot `has_key == false`, `turns.is_empty()`, `provider == OpenAi`, `model.provider() == OpenAi`.
  - `switching_provider_mid_stream_stops_the_reply`: send `[slow] go`, wait for `Streaming`, `set_provider(OpenAi)`, wait 1s, snapshot has no turns and status `Ready`. (Review Focus 2)
  - `models_from_the_other_provider_are_rejected`: Anthropic visitor, `set_model(<an OpenAI model>)` returns false and the model is unchanged.
- [ ] **Step 3: Run** `.github/scripts/cargo-retry.sh cargo test -p site live::` → new tests FAIL (compile errors count).
- [ ] **Step 4: Implement** the interfaces above. Update existing call sites and tests to pass `Provider::Anthropic` where needed.
- [ ] **Step 5: Run** `.github/scripts/cargo-retry.sh cargo test --workspace` → all pass.
- [ ] **Step 6: Commit** `"Add providers, per-provider models and key checks"`.

---

### Task 3: Put the Anthropic protocol behind an adapter

Refactor only. Every existing Anthropic test passes unchanged in what it asserts.

**Files:**
- Create: `site/src/live/anthropic.rs` (from `api.rs`: request body, `Accumulator`, error parsing, history merge, API version, production URL)
- Modify: `site/src/live/api.rs` (keeps `Client`, `Stream`, `Endpoint`, `ApiError`, `ErrorKind`, `Update`, `MAX_TOKENS`, `read_capped`), `site/src/live/provider.rs`, `site/src/live/chat.rs`
- Test: existing `site/src/live/tests.rs`, plus `anthropic.rs` unit tests moved from `api.rs`

**Interfaces:**
- Produces, in `provider.rs` (dispatching by `match self` to the adapter modules):
  - `pub enum Outcome { ToolUse, MaxTokens, Refusal, Done }`
  - `pub struct ToolCall { pub id: String, pub name: String, pub input: Result<Value, String> }` (`Err` holds raw text that was not valid JSON)
  - `pub struct Finished { pub outcome: Outcome, pub calls: Vec<ToolCall>, pub items: Vec<Value> }`. `items` are the history items for the model's output this round, to append exactly as given.
  - `pub enum StreamState { Anthropic(anthropic::Accumulator), OpenAi(openai::Accumulator) }` with `apply(&mut self, event: &sse::Event, key: &ApiKey) -> Result<Vec<Update>, ApiError>` and `finished(&self) -> Option<Finished>` (Some once the provider's terminal event arrived).
  - `impl Provider`: `user_items(self, text: &str) -> Vec<Value>`, `interrupted_items(self, text: &str) -> Vec<Value>` (assistant text kept after stop/error), `tool_result_items(self, results: &[(ToolCall, String, bool)]) -> Vec<Value>` (call, output text, is_error), `request_body(self, model: Model, history: &[Value]) -> Value`, `prepare_history(self, items: Vec<Value>) -> Vec<Value>` (Anthropic: today's `push_merged`; OpenAI: identity), `stream_state(self) -> StreamState`, `parse_error(self, status: Option<u16>, body: &str, retry_after: Option<u64>, key: &ApiKey) -> ApiError`.
- Changes in `api.rs`:
  - `Update::BlockStopped { index, tool_input: Option<String> }`: the adapter supplies the pretty-printed (or raw invalid) tool input, so `chat.rs` no longer reads provider JSON.
  - `Endpoint` becomes per provider: `Endpoint::production(p)`, `Endpoint::from_env(p)` (debug env vars `DEMO_ANTHROPIC_BASE_URL`, `DEMO_OPENAI_BASE_URL`), `Endpoint::local(url)`. `pub struct Endpoints { pub anthropic: Endpoint, pub openai: Endpoint }`, `Demo::new(endpoints: Endpoints)`.
  - `Client::stream(&self, provider: Provider, key: &ApiKey, body: &Value) -> Result<Stream, ApiError>`: path and auth headers come from the provider (Anthropic: `/v1/messages`, `x-api-key`, `anthropic-version`). Header stays `set_sensitive(true)`. `https_only` per endpoint.
  - `ApiError::explain(&self, model: Model) -> String` uses `model.provider().label()` wherever it names the provider.
- Consumes: `Provider`, `Model` from Task 2.

- [ ] **Step 1: Add unit test in `anthropic.rs`** `tool_input_arrives_with_block_stop`: feed the content_block_start/input_json_delta/content_block_stop events for a `tool_use` block; the last update is `BlockStopped { index: 0, tool_input: Some(pretty) }` where `pretty == serde_json::to_string_pretty(&json!({"expression":"2+2"}))`.
- [ ] **Step 2: Run** it → FAIL (no such field).
- [ ] **Step 3: Move the code and route `chat.rs` through the `Provider` methods.** `generate` matches on `Outcome`; `run_tools` takes `&[ToolCall]`; `history()` builds items with `provider.user_items` and passes them through `provider.prepare_history`; `interrupt` uses `provider.interrupted_items`. Store the provider on `Job` (copied at start, so a later `set_provider` cannot change a running task's protocol).
- [ ] **Step 4: Run** `.github/scripts/cargo-retry.sh cargo test --workspace` → all pass. `grep -n '"tool_use"\|"content_block\|anthropic-version' site/src/live/chat.rs` → no matches.
- [ ] **Step 5: Commit** `"Move the Anthropic protocol behind a provider adapter"`.

---

### Task 4: OpenAI adapter and fake

**Files:**
- Create: `site/src/live/openai.rs`
- Modify: `site/src/live/fake.rs` (OpenAI mode), `site/src/live/provider.rs` (dispatch arms), `site/src/main.rs` (fake serves both when `DEMO_FAKE_API=1`)
- Test: `site/src/live/openai.rs` unit tests, `site/src/live/tests.rs`

**Interfaces:**
- Consumes: everything Task 3 produced.
- Produces: `openai::Accumulator` and the `Provider::OpenAi` arms. `FakeApi::url()` serves both protocols by path (`/v1/messages` Anthropic, the OpenAI path for OpenAI); `Recorded` gains `authorization: String` (deliberately still not `Debug`).

Before writing code, read OpenAI's official docs (https://developers.openai.com/api/docs, the Responses API reference and its streaming-events page, the reasoning guide, the function-calling guide, the error-codes page). Use the API OpenAI recommends for new work that streams reasoning summaries and supports function tools (expected: the Responses API, `POST /v1/responses`). Copy event names and fields from the reference, not memory. Record the doc URLs in the module comment. Decisions this plan fixes:

- Auth: `Authorization: Bearer <key>`, header `set_sensitive(true)`. No organization or project headers.
- Request: `stream: true`, `store: false` (nothing kept on OpenAI's side), the system prompt as instructions, `max_output_tokens: MAX_TOKENS`, reasoning summaries requested (`auto`), encrypted reasoning included so it can be sent back with `store: false`, the two tools from `tools::definitions()` converted to OpenAI function tools (same names, descriptions and JSON schemas; write the converter, don't duplicate the definitions).
- History items appended as the API returned them (reasoning, message, function_call), then `function_call_output` items keyed by `call_id`.
- Stream to `Update`: reasoning summary part start → `ThinkingStarted`; summary delta → `Thinking`; output text start/delta → `TextStarted`/`Text`; function call item added → `ToolStarted`; arguments delta → `ToolInput`; item done → `BlockStopped` (with pretty input for function calls). Index = the output item index plus summary index mapped to a dense counter, so `chat.rs` sees indexes 0,1,2… A reasoning item that finishes with no summary text emits no `ThinkingStarted` at all.
- Terminal: `response.completed` → `Outcome::ToolUse` if any function call, else `Done`; `response.incomplete` with max-output-tokens reason → `MaxTokens`; a refusal content part → `Refusal`; `response.failed` / `error` event → `ApiError::Api` via `parse_error`.
- Error mapping (by status and `error.code`/`error.type`): 400 → `InvalidRequest`; 401 → `Authentication`; 403 → `Permission`; 404 (including `model_not_found`) → `NotFound`; 429 with code `insufficient_quota` → `Billing`; other 429 → `RateLimit`; 500 → `Api`; 503 → `Overloaded`; 504/timeouts → `Timeout`. Message scrubbed of the key exactly as Anthropic's is.
- Fake OpenAI mode: same trigger words as the Anthropic fake (`[401]` `[402]`→429 insufficient_quota, `[429]`, `[500]`, `[503]`, `[midstream]`, `[slow]`, `[refusal]`, `[maxtokens]`, `[text]`, `[nothink]` → no reasoning summary, `[badargs]` → function call with invalid JSON arguments, `[echokey]` → 401 whose message contains the request's key), and tool-result follow-up the same way. Event shapes copied from the OpenAI reference.

- [ ] **Step 1: Write failing tests in `tests.rs`**, a parallel OpenAI set using `const OPENAI_KEY: &str = "sk-proj-visitor-c-0123456789abcdefghijklmnop";` and a `visitor_with(demo, Provider::OpenAi, OPENAI_KEY)` helper:
  - `openai_streams_reasoning_a_tool_call_and_text`: parts are `[Thinking, Text, Tool(Completed, output contains "4"), Thinking, Text]` for `what is 2+2`; second request body's input ends with a `function_call` then `function_call_output` with matching `call_id`; recorded `authorization == "Bearer " + OPENAI_KEY`; recorded body has `store == false`.
  - `openai_api_errors_render_as_notices`: `[401]` → `key_problem == true`; `[429]` → notice mentions rate limit; `[500]`, `[503]` → error notices.
  - `openai_no_credit_is_a_billing_notice`: `[402]` (fake sends 429 `insufficient_quota`) → notice text matches the Anthropic billing wording, not the rate-limit wording. (Review Focus 4)
  - `openai_without_reasoning_shows_no_thinking_part`: `[nothink] hi` → no `Part::Thinking` in the reply. (Review Focus 3)
  - `openai_invalid_tool_arguments_become_a_tool_error`: `[badargs]` → tool part status `Error`, next request contains a `function_call_output` whose output starts with `INVALID_JSON`.
  - `openai_error_text_never_contains_the_key`: `[echokey]` → notice text does not contain `OPENAI_KEY` nor its last 20 chars. (Review Focus 5)
  - `openai_stop_mid_stream_keeps_text`, `openai_stop_during_tool_call_cancels_it`, `openai_regenerate_adds_a_branch`, `openai_midstream_error_keeps_text`: same assertions as the Anthropic versions; after a stop, the next request's input is accepted by the fake (the fake rejects malformed input items with 400, so this proves `interrupted_items` is well formed).
  - `the_key_never_appears_in_pages_or_printed_state` extended to an OpenAI visitor.
- [ ] **Step 2: Run** `.github/scripts/cargo-retry.sh cargo test -p site live::tests::openai` → FAIL.
- [ ] **Step 3: Implement** `openai.rs`, the provider arms, and the fake's OpenAI mode as specified above.
- [ ] **Step 4: Run** `.github/scripts/cargo-retry.sh cargo test --workspace` → all pass.
- [ ] **Step 5: Commit** `"Add the OpenAI adapter and fake"`.

---

### Task 5: The page

**Files:**
- Modify: `site/src/live/view.rs`, `site/src/shell.rs`, `site/src/catalog.rs` (catalog text, `llms.txt`), `site/src/examples/chat_app.rs` (empty-state link text), `site/src/blocks*` card text if it names Claude, `README.md`
- Test: `site/src/live/tests.rs`

**Interfaces:**
- Consumes: `Provider`, `Model::for_provider`, `Visitor::set_provider`, `Snapshot.provider`, `KeyError::WrongProvider`.
- Produces: procedure `set_provider(cx: &Cx, provider: String) -> Result<bool>` that accepts only `Provider::from_id` values.

Page behavior:
- Title "Live model demo". Key form starts with a two-option segmented choice "Anthropic | OpenAI" (radio group with a visible label "Provider", keyboard operable). Choosing one calls `set_provider`.
- Key field label "{Provider} API key", placeholder shows that provider's prefix, help link to `provider.keys_url()`, text recommends a restricted, low-limit key.
- Disclosure text, provider-neutral: the key is sent to the topcoat-prompt server, held only in memory, never stored, used only for this demo, and every message is billed to the visitor's {Provider} account. Limits listed as now.
- Chat header: model select lists only `Model::for_provider(snapshot.provider)`; badge "Billed to your {Provider} account"; a "Change provider" control next to "Clear key" that returns to the key form (clears key and conversation, via `set_provider`/`clear_key`).
- System prompt and notices say "the model" or the model label, not "Claude", where the provider could be OpenAI.
- Sidebar entry "Live model demo" → `/full/live`. `/blocks` card, fake demo empty state, catalog and `llms.txt` say "Anthropic or OpenAI key".

- [ ] **Step 1: Write failing tests in `tests.rs`:**
  - `the_key_form_offers_both_providers`: `GET /full/live` HTML contains `Anthropic`, `OpenAI`, and the label `Anthropic API key` (default provider); after `set_provider(OpenAi)` on that session, the page contains `OpenAI API key` and the OpenAI keys URL, and no `platform.claude.com`.
  - `the_model_list_follows_the_provider`: OpenAI visitor with a key, page HTML contains every `Model::for_provider(OpenAi)` id as an `<option value>` and no Anthropic model id.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement** the page as described, using existing components only (no registry changes).
- [ ] **Step 4: Run** the full check list from Global Constraints → all green.
- [ ] **Step 5: Browser pass** with `DEMO_FAKE_API=1 PORT=3491 topcoat dev -p site` (leave the user's server on 3417 and the user's tabs alone; open and close your own tab). In light and dark, at desktop and 390px width, for each provider: enter a key, suggestion → reasoning → tool Pending → Completed → text, Stop, Regenerate (2/2), model switch, `[401]` notice with Change key, Clear key, Change provider, swapped key message, `[nothink]` shows no reasoning block, `/full/claude` redirects. Check: no console errors; the key is absent from the DOM, local/session storage, request URLs and server output. Keyboard: provider choice, key field, Save, model select reachable and operable with Tab/Space/Enter/arrows.
- [ ] **Step 6: Commit** `"Offer Anthropic or OpenAI on the live demo page"`.

---

### Task 6: Pull request

- [ ] **Step 1:** `git push -u origin openai-provider`.
- [ ] **Step 2:** `gh pr create` with: what changed, the history-format refinement above, the OpenAI API and models chosen with doc links, the key-handling paths for OpenAI (same list as PR #10, with `Authorization: Bearer` replacing `x-api-key`), test results, and "Not verified without real keys: that api.openai.com accepts the request exactly, real error bodies, TLS from the Docker image." End with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- [ ] **Step 3:** wait for CI (`gh pr checks --watch`), fix failures, report the PR URL. Do not merge.
