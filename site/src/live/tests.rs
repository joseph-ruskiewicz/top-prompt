//! End-to-end tests of the real-model demo against the fake API: streaming, tool
//! calls, errors, stopping, isolation between visitors, limits, and where the key
//! does and does not appear.

use std::{net::IpAddr, sync::Arc, time::Duration};

use serde_json::{Value, json};
use tokio::time::{Instant, timeout};
use topcoat::{
    router::{Body, request::Request, response::Response},
    session::{Token, TokenHash},
};

use super::{
    anthropic::API_VERSION,
    api::{Endpoint, Endpoints, MAX_TOKENS},
    chat::{
        Demo, IDLE_TIMEOUT, IP_REQUESTS, MAX_TURNS, Part, ReplyState, ReplyView, Snapshot,
        VISITOR_REQUESTS, Visitor,
    },
    fake::FakeApi,
    provider::{Model, Provider},
};
use crate::components::{prompt_input::PromptStatus, tool_call::ToolStatus};

const KEY_A: &str = "sk-ant-api03-visitor-a-0123456789abcdefghijklmnop";
const KEY_B: &str = "sk-ant-api03-visitor-b-zyxwvutsrqponmlkjihgfedcba";
const OPENAI_KEY: &str = "sk-proj-visitor-c-0123456789abcdefghijklmnop";

async fn setup() -> (FakeApi, Demo) {
    let fake = FakeApi::start().await.unwrap();
    let demo = Demo::new(Endpoints {
        anthropic: Endpoint::local(fake.url()),
        openai: Endpoint::local(fake.url()),
    });
    (fake, demo)
}

fn session() -> TokenHash {
    Token::random().hash()
}

/// A new visitor with `key` set.
fn visitor(demo: &Demo, key: &str) -> Arc<Visitor> {
    let visitor = demo.visitor(&session()).unwrap();
    visitor.set_key(key).unwrap();
    visitor
}

/// Waits until no reply is being generated, then returns the snapshot.
async fn settle(visitor: &Visitor) -> Snapshot {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let mut changed = visitor.subscribe();
        let snapshot = visitor.snapshot();
        if snapshot.status == PromptStatus::Ready {
            return snapshot;
        }
        timeout(deadline - Instant::now(), changed.recv())
            .await
            .expect("the reply finishes")
            .ok();
    }
}

/// Waits until `check` holds for the visitor's snapshot.
async fn wait_for(visitor: &Visitor, check: impl Fn(&Snapshot) -> bool) -> Snapshot {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let mut changed = visitor.subscribe();
        let snapshot = visitor.snapshot();
        if check(&snapshot) {
            return snapshot;
        }
        timeout(deadline - Instant::now(), changed.recv())
            .await
            .expect("the condition holds in time")
            .ok();
    }
}

fn last_reply(snapshot: &Snapshot) -> ReplyView {
    let turn = snapshot.turns.last().expect("a turn");
    turn.replies[turn.current].clone()
}

fn ip(last: u8) -> Option<IpAddr> {
    Some(IpAddr::from([203, 0, 113, last]))
}

/// The roles of a message list, which must alternate starting with the user.
fn roles(messages: &[Value]) -> Vec<&str> {
    messages
        .iter()
        .map(|message| message["role"].as_str().unwrap())
        .collect()
}

fn assert_alternates(messages: &[Value]) {
    for (index, role) in roles(messages).into_iter().enumerate() {
        let expected = if index % 2 == 0 { "user" } else { "assistant" };
        assert_eq!(role, expected, "message {index} in {messages:#?}");
    }
}

#[tokio::test]
async fn streams_thinking_a_tool_call_and_text() {
    let (fake, demo) = setup().await;
    let visitor = visitor(&demo, KEY_A);
    assert!(visitor.send(&demo, "What is 12 * 12?", ip(1)));

    let snapshot = settle(&visitor).await;
    let reply = last_reply(&snapshot);
    assert_eq!(reply.state, ReplyState::Done);
    assert!(reply.notice.is_none());
    let kinds: Vec<&str> = reply
        .parts
        .iter()
        .map(|part| match part {
            Part::Thinking { .. } => "thinking",
            Part::Tool { .. } => "tool",
            Part::Text(_) => "text",
        })
        .collect();
    assert_eq!(kinds, ["thinking", "text", "tool", "thinking", "text"]);
    let Part::Tool {
        name,
        status,
        input,
        output,
        error,
    } = &reply.parts[2]
    else {
        unreachable!()
    };
    assert_eq!(name, "calculate");
    assert_eq!(*status, ToolStatus::Completed);
    assert_eq!(
        serde_json::from_str::<Value>(input).unwrap(),
        json!({"expression": "12 * 12"})
    );
    assert_eq!(output.as_deref(), Some("144"));
    assert!(!error);
    let Part::Thinking {
        text,
        streaming,
        secs,
        ..
    } = &reply.parts[0]
    else {
        unreachable!()
    };
    assert!(text.contains("calculate"));
    assert!(!streaming);
    assert!(secs.is_some());
    assert!(reply.text().contains("`144`"));

    // Two requests: the question, then the tool result.
    let requests = fake.requests();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert_eq!(request.api_key, KEY_A);
        assert_eq!(request.version, API_VERSION);
        assert_eq!(request.path, "/v1/messages");
        assert_eq!(request.body["model"], "claude-sonnet-5-5");
        assert_eq!(request.body["max_tokens"], MAX_TOKENS);
        assert_eq!(request.body["stream"], true);
        assert_eq!(
            request.body["thinking"],
            json!({"type": "adaptive", "display": "summarized"})
        );
    }
    let second = requests[1].body["messages"].as_array().unwrap();
    assert_alternates(second);
    assert_eq!(second.len(), 3);
    // The assistant turn goes back exactly as streamed, thinking signature included.
    assert_eq!(
        second[1],
        json!({"role": "assistant", "content": [
            {
                "type": "thinking",
                "thinking": "The user wants a number. I'll use the calculate tool on `12 * 12`.",
                "signature": "fake-signature-0",
            },
            { "type": "text", "text": "Let me work that out." },
            {
                "type": "tool_use",
                "id": "toolu_fake2",
                "name": "calculate",
                "input": { "expression": "12 * 12" },
            },
        ]})
    );
    assert_eq!(
        second[2],
        json!({"role": "user", "content": [
            { "type": "tool_result", "tool_use_id": "toolu_fake2", "content": "144" },
        ]})
    );

    // The conversation continues from the full history.
    let history = visitor.api_history();
    assert_eq!(roles(&history), ["user"]);
    assert!(visitor.send(&demo, "[text] thanks", ip(1)));
    settle(&visitor).await;
    let third = fake.requests()[2].body["messages"].clone();
    let third = third.as_array().unwrap();
    assert_alternates(third);
    assert_eq!(third.len(), 5);
    assert_eq!(third[1], second[1]);
}

#[tokio::test]
async fn searches_the_component_catalog() {
    let (_fake, demo) = setup().await;
    let visitor = visitor(&demo, KEY_A);
    assert!(visitor.send(&demo, "Which component shows reasoning?", ip(1)));
    let reply = last_reply(&settle(&visitor).await);
    let tool = reply
        .parts
        .iter()
        .find_map(|part| match part {
            Part::Tool {
                name,
                status,
                output,
                ..
            } => Some((name.clone(), *status, output.clone().unwrap_or_default())),
            _ => None,
        })
        .unwrap();
    assert_eq!(tool.0, "search_components");
    assert_eq!(tool.1, ToolStatus::Completed);
    assert!(tool.2.contains("\"name\": \"reasoning\""), "{}", tool.2);
}

#[tokio::test]
async fn api_errors_render_as_notices() {
    let (_fake, demo) = setup().await;
    let cases = [
        ("[401]", "(401)", "rejected", true),
        ("[402]", "(402)", "billing", false),
        ("[429]", "(429)", "7 seconds", false),
        ("[500]", "(500)", "internal error", false),
        ("[529]", "(529)", "overloaded", false),
    ];
    for (marker, code, phrase, key_problem) in cases {
        let visitor = visitor(&demo, KEY_A);
        assert!(visitor.send(&demo, &format!("{marker} hello"), ip(2)));
        let reply = last_reply(&settle(&visitor).await);
        assert_eq!(reply.state, ReplyState::Failed, "{marker}");
        let notice = reply.notice.expect("a notice");
        assert!(notice.error);
        assert!(notice.text.contains(code), "{marker}: {}", notice.text);
        assert!(notice.text.contains(phrase), "{marker}: {}", notice.text);
        assert_eq!(notice.key_problem, key_problem, "{marker}");
        assert!(!notice.text.contains(KEY_A));
        // Nothing reached the history but the question.
        assert_eq!(roles(&visitor.api_history()), ["user"]);
    }
}

#[tokio::test]
async fn an_error_mid_stream_keeps_the_text_so_far() {
    let (fake, demo) = setup().await;
    let visitor = visitor(&demo, KEY_A);
    assert!(visitor.send(&demo, "[midstream] go", ip(3)));
    let reply = last_reply(&settle(&visitor).await);
    assert_eq!(reply.state, ReplyState::Failed);
    let notice = reply.notice.clone().unwrap();
    assert!(notice.text.contains("overloaded"), "{}", notice.text);
    assert!(notice.text.contains("mid-stream"), "{}", notice.text);
    assert_eq!(reply.text(), "This reply starts, and then");

    // The partial text is history now, so the next request still alternates.
    assert!(visitor.send(&demo, "[text] again", ip(3)));
    settle(&visitor).await;
    let messages = fake.requests().last().unwrap().body["messages"].clone();
    let messages = messages.as_array().unwrap();
    assert_alternates(messages);
    assert_eq!(
        messages[1],
        json!({"role": "assistant", "content": [{"type": "text", "text": "This reply starts, and then"}]})
    );
}

#[tokio::test]
async fn stopping_keeps_what_arrived_and_ends_the_stream() {
    let (fake, demo) = setup().await;
    let visitor = visitor(&demo, KEY_A);
    assert!(visitor.send(&demo, "[slow] go", ip(4)));
    wait_for(&visitor, |snapshot| {
        last_reply(snapshot).text().split_whitespace().count() >= 6
    })
    .await;
    visitor.stop();

    let stopped = visitor.snapshot();
    assert_eq!(stopped.status, PromptStatus::Ready);
    let reply = last_reply(&stopped);
    assert_eq!(reply.state, ReplyState::Stopped);
    let text = reply.text();
    // The aborted task makes no further changes.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(last_reply(&visitor.snapshot()).text(), text);
    assert!(text.len() < "This reply streams slowly so there is time to stop it. ".len() * 4);

    assert!(visitor.send(&demo, "[text] and now?", ip(4)));
    assert_eq!(last_reply(&settle(&visitor).await).state, ReplyState::Done);
    let messages = fake.requests().last().unwrap().body["messages"].clone();
    let messages = messages.as_array().unwrap();
    assert_alternates(messages);
    assert_eq!(messages[1]["content"][0]["text"], text.as_str());
}

#[tokio::test]
async fn stopping_during_a_tool_call_cancels_it() {
    let (fake, demo) = setup().await;
    let visitor = visitor(&demo, KEY_A);
    assert!(visitor.send(&demo, "2 + 2", ip(5)));
    wait_for(&visitor, |snapshot| {
        last_reply(snapshot)
            .parts
            .iter()
            .any(|part| matches!(part, Part::Tool { .. }))
    })
    .await;
    visitor.stop();
    let reply = last_reply(&visitor.snapshot());
    let Some(Part::Tool { status, .. }) = reply
        .parts
        .iter()
        .find(|part| matches!(part, Part::Tool { .. }))
    else {
        unreachable!()
    };
    assert_eq!(*status, ToolStatus::Error);

    assert!(visitor.send(&demo, "[text] ok", ip(5)));
    settle(&visitor).await;
    let messages = fake.requests().last().unwrap().body["messages"].clone();
    let messages = messages.as_array().unwrap();
    assert_alternates(messages);
    // No tool_use without its result went back.
    assert!(!messages.iter().any(|message| {
        message["content"]
            .as_array()
            .unwrap()
            .iter()
            .any(|block| block["type"] == "tool_use")
    }));
}

#[tokio::test]
async fn concurrent_visitors_stay_isolated() {
    let (fake, demo) = setup().await;
    let alice = visitor(&demo, KEY_A);
    let bob = visitor(&demo, KEY_B);
    assert!(alice.send(&demo, "What is 111 + 1?", ip(6)));
    assert!(bob.send(&demo, "What is 222 + 2?", ip(7)));
    assert_eq!(alice.snapshot().status, PromptStatus::Submitted);
    assert_eq!(bob.snapshot().status, PromptStatus::Submitted);

    let (alice_view, bob_view) = tokio::join!(settle(&alice), settle(&bob));
    assert_eq!(alice_view.turns.len(), 1);
    assert_eq!(bob_view.turns.len(), 1);
    assert_eq!(alice_view.turns[0].user, "What is 111 + 1?");
    assert_eq!(bob_view.turns[0].user, "What is 222 + 2?");
    assert!(last_reply(&alice_view).text().contains("`112`"));
    assert!(last_reply(&bob_view).text().contains("`224`"));

    // Each visitor's requests carried only their own key and conversation.
    let requests = fake.requests();
    assert_eq!(requests.len(), 4);
    for request in &requests {
        let body = request.body.to_string();
        match request.api_key.as_str() {
            KEY_A => assert!(body.contains("111 + 1") && !body.contains("222 + 2")),
            KEY_B => assert!(body.contains("222 + 2") && !body.contains("111 + 1")),
            _ => panic!("a request with an unknown key"),
        }
    }

    // Stopping or clearing one leaves the other alone.
    bob.clear_key();
    assert!(!bob.snapshot().has_key);
    assert!(alice.snapshot().has_key);
    assert_eq!(alice.snapshot().turns.len(), 1);
}

#[tokio::test]
async fn sessions_map_to_their_own_visitors() {
    let (_fake, demo) = setup().await;
    let first = session();
    let second = session();
    let a = demo.visitor(&first).unwrap();
    a.set_key(KEY_A).unwrap();
    assert!(Arc::ptr_eq(&a, &demo.visitor(&first).unwrap()));
    assert!(!demo.visitor(&second).unwrap().snapshot().has_key);
}

#[tokio::test]
async fn regenerate_adds_a_branch() {
    let (fake, demo) = setup().await;
    let visitor = visitor(&demo, KEY_A);
    assert!(visitor.send(&demo, "[text] one", ip(8)));
    settle(&visitor).await;
    assert!(visitor.regenerate(&demo, ip(8)));
    let snapshot = settle(&visitor).await;
    assert_eq!(snapshot.turns[0].replies.len(), 2);
    assert_eq!(snapshot.turns[0].current, 1);
    // The regenerated request is the question alone.
    let messages = fake.requests().last().unwrap().body["messages"].clone();
    assert_eq!(roles(messages.as_array().unwrap()), ["user"]);

    assert!(visitor.select_branch(snapshot.turns[0].id, 0));
    assert_eq!(visitor.snapshot().turns[0].current, 0);
    assert!(!visitor.select_branch(snapshot.turns[0].id, 5));
}

#[tokio::test]
async fn rejects_input_outside_the_limits() {
    let (_fake, demo) = setup().await;
    let visitor = demo.visitor(&session()).unwrap();
    // No key yet.
    assert!(!visitor.send(&demo, "hello", ip(9)));
    assert!(visitor.set_key("not a key").is_err());
    assert!(visitor.set_key("sk-proj-0123456789abcdef").is_err());
    visitor.set_key(KEY_A).unwrap();

    assert!(!visitor.send(&demo, "   ", ip(9)));
    assert!(!visitor.send(&demo, &"a".repeat(4001), ip(9)));
    assert!(Model::from_id("claude-3-opus").is_none());
    assert!(visitor.set_model(Model::Haiku));
    assert_eq!(visitor.snapshot().model, Model::Haiku);

    // One reply at a time.
    assert!(visitor.send(&demo, "[slow] go", ip(9)));
    assert!(!visitor.send(&demo, "[text] two", ip(9)));
    assert!(!visitor.set_model(Model::Opus));
    visitor.stop();
}

#[tokio::test]
async fn switching_provider_clears_key_and_conversation() {
    let (_fake, demo) = setup().await;
    let visitor = visitor(&demo, KEY_A);
    assert!(visitor.send(&demo, "[text] hi", ip(30)));
    assert_eq!(settle(&visitor).await.turns.len(), 1);

    assert!(visitor.set_provider(Provider::OpenAi));
    let snapshot = visitor.snapshot();
    assert!(!snapshot.has_key);
    assert!(snapshot.turns.is_empty());
    assert_eq!(snapshot.provider, Provider::OpenAi);
    assert_eq!(snapshot.model.provider(), Provider::OpenAi);
    assert_eq!(snapshot.model, Provider::OpenAi.default_model());
    // Choosing the same provider again changes nothing.
    assert!(!visitor.set_provider(Provider::OpenAi));
    // Keys are now checked against OpenAI's shape.
    assert!(visitor.set_key(KEY_A).is_err());
}

#[tokio::test]
async fn switching_provider_mid_stream_stops_the_reply() {
    let (_fake, demo) = setup().await;
    let visitor = visitor(&demo, KEY_A);
    assert!(visitor.send(&demo, "[slow] go", ip(31)));
    wait_for(&visitor, |snapshot| {
        snapshot.status == PromptStatus::Streaming
    })
    .await;
    assert!(visitor.set_provider(Provider::OpenAi));
    assert!(!visitor.snapshot().has_key);
    // No generation is left running: a model change and a new message are both
    // refused while one is.
    assert!(visitor.set_model(Model::Luna));
    visitor.set_key(OPENAI_KEY).unwrap();
    assert!(visitor.send(&demo, "[text] again", ip(31)));
    visitor.stop();
    // The old task makes no late changes, so the new turn holds only its own
    // stopped, empty reply.
    tokio::time::sleep(Duration::from_secs(1)).await;
    let snapshot = visitor.snapshot();
    assert_eq!(snapshot.status, PromptStatus::Ready);
    assert_eq!(snapshot.turns.len(), 1);
    assert_eq!(snapshot.turns[0].user, "[text] again");
    let reply = last_reply(&snapshot);
    assert_eq!(reply.state, ReplyState::Stopped);
    assert_eq!(reply.model, Model::Luna);
    assert!(reply.parts.is_empty(), "{:?}", reply.parts);
}

#[tokio::test]
async fn models_from_the_other_provider_are_rejected() {
    let (_fake, demo) = setup().await;
    let visitor = visitor(&demo, KEY_A);
    let before = visitor.snapshot().model;
    let openai = Model::for_provider(Provider::OpenAi).next().unwrap();
    assert!(!visitor.set_model(openai));
    assert_eq!(visitor.snapshot().model, before);
    assert_eq!(before.provider(), Provider::Anthropic);
}

#[tokio::test]
async fn caps_turns_and_requests() {
    let (fake, demo) = setup().await;
    let visitor = visitor(&demo, KEY_A);
    // Errors come back at once, so each turn is one quick request.
    for turn in 0..MAX_TURNS {
        assert!(visitor.send(&demo, "[401] hi", ip(10)), "turn {turn}");
        settle(&visitor).await;
    }
    assert_eq!(MAX_TURNS, VISITOR_REQUESTS);
    let snapshot = visitor.snapshot();
    assert!(snapshot.full);
    assert!(!visitor.send(&demo, "[401] one more", ip(10)));

    // Regenerating is still allowed, but the request limit stops it before any
    // request is made.
    let before = fake.requests().len();
    assert!(visitor.regenerate(&demo, ip(10)));
    let reply = last_reply(&settle(&visitor).await);
    assert_eq!(fake.requests().len(), before);
    assert!(
        reply
            .notice
            .unwrap()
            .text
            .contains("requests in the last 10 minutes")
    );
}

#[tokio::test]
async fn caps_requests_per_address() {
    let (fake, demo) = setup().await;
    let shared = ip(11);
    let mut sent = 0;
    while sent < IP_REQUESTS {
        let visitor = visitor(&demo, KEY_A);
        for _ in 0..10 {
            assert!(visitor.send(&demo, "[401] hi", shared));
            settle(&visitor).await;
            sent += 1;
        }
    }
    let visitor = visitor(&demo, KEY_A);
    assert!(visitor.send(&demo, "[401] hi", shared));
    let reply = last_reply(&settle(&visitor).await);
    assert!(reply.notice.unwrap().text.contains("This network"));
    assert_eq!(fake.requests().len(), IP_REQUESTS);
    // Another address is not affected.
    assert!(visitor.send(&demo, "[401] hi", ip(12)));
    assert!(
        last_reply(&settle(&visitor).await)
            .notice
            .unwrap()
            .text
            .contains("(401)")
    );
}

#[tokio::test]
async fn idle_visitors_are_dropped_with_their_key() {
    let (_fake, demo) = setup().await;
    let session = session();
    let visitor = demo.visitor(&session).unwrap();
    visitor.set_key(KEY_A).unwrap();
    let mut changed = visitor.subscribe();

    demo.sweep_now(IDLE_TIMEOUT);
    assert_eq!(demo.visitor_count(), 1, "a fresh visitor stays");

    visitor.age(IDLE_TIMEOUT + Duration::from_secs(1));
    demo.sweep_now(IDLE_TIMEOUT);
    assert_eq!(demo.visitor_count(), 0);
    // The page watching it is woken, and finds a visitor without a key.
    changed.try_recv().unwrap();
    assert!(!demo.visitor(&session).unwrap().snapshot().has_key);
}

/// Builds the site's router around `demo`, as `main` does. Tests have no asset
/// bundle, so the router gets a catalog of the assets this binary references, as if
/// they were hosted elsewhere; pages render their URLs without the files.
fn site(demo: &Demo) -> topcoat::router::Router {
    use topcoat::asset::{
        AssetCatalog, AssetConfig, MANIFEST_VERSION, Manifest, ManifestEntry, RawAsset,
    };

    let binary = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    let assets = RawAsset::find_in_binary(&binary)
        .into_iter()
        .enumerate()
        .map(|(index, asset)| ManifestEntry {
            id: asset.id(),
            file: format!("asset-{index}"),
            hash: String::new(),
            content_type: "application/octet-stream".to_owned(),
        })
        .collect();
    let catalog = AssetCatalog::from(Manifest {
        version: MANIFEST_VERSION,
        assets,
    });
    crate::router(
        demo.clone(),
        AssetConfig::hosted_at("https://assets.invalid", catalog),
    )
}

async fn get(
    router: &topcoat::router::Router,
    path: &str,
    cookie: Option<&str>,
) -> (Response, String) {
    let mut request = Request::builder().uri(path);
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    let mut response = router.handle(request.body(Body::empty()).unwrap()).await;
    let body = std::mem::replace(response.body_mut(), Body::empty());
    let bytes = topcoat::router::to_bytes(body, 8 * 1024 * 1024)
        .await
        .unwrap();
    (response, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn old_address_redirects_to_live() {
    let (_fake, demo) = setup().await;
    let router = site(&demo);
    let (response, _) = get(&router, "/full/claude", None).await;
    assert_eq!(response.status(), 308);
    assert_eq!(response.headers().get("location").unwrap(), "/full/live");
    let (response, _) = get(&router, "/full/live", None).await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn the_first_load_sets_a_strict_session_cookie() {
    let (_fake, demo) = setup().await;
    let router = site(&demo);
    let (response, body) = get(&router, "/full/live", None).await;
    assert_eq!(response.status(), 200);
    let cookie = response
        .headers()
        .get("set-cookie")
        .expect("a session cookie")
        .to_str()
        .unwrap()
        .to_owned();
    assert!(cookie.starts_with("__Host-demo="), "{cookie}");
    for attribute in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/"] {
        assert!(cookie.contains(attribute), "{cookie}");
    }
    // A browser-session cookie: no lifetime, no domain.
    for attribute in ["Max-Age", "Expires", "Domain"] {
        assert!(!cookie.contains(attribute), "{cookie}");
    }
    // The key form says what happens to the key.
    for text in [
        "sent to the topcoat-prompt server",
        "held only in the server",
        "never stored",
        "used only for this demo",
        "https://platform.claude.com/settings/keys",
        "type=\"password\"",
    ] {
        assert!(body.contains(text), "missing {text:?}");
    }

    // The same cookie finds the same visitor on the next load.
    let token = cookie.split(';').next().unwrap();
    let (again, _) = get(&router, "/full/live", Some(token)).await;
    assert!(again.headers().get("set-cookie").is_none());
}

/// A router, a cookie for a session and the visitor behind it.
fn page_visitor(demo: &Demo) -> (topcoat::router::Router, String, Arc<Visitor>) {
    let token = Token::random();
    let cookie = format!("__Host-demo={}", token.encode());
    let visitor = demo.visitor(&token.hash()).unwrap();
    (site(demo), cookie, visitor)
}

#[tokio::test]
async fn the_key_form_offers_both_providers() {
    let (_fake, demo) = setup().await;
    let (router, cookie, visitor) = page_visitor(&demo);

    let (_, page) = get(&router, "/full/live", Some(&cookie)).await;
    for text in ["Anthropic", "OpenAI", "Anthropic API key"] {
        assert!(page.contains(text), "missing {text:?}");
    }

    assert!(visitor.set_provider(Provider::OpenAi));
    let (_, page) = get(&router, "/full/live", Some(&cookie)).await;
    assert!(page.contains("OpenAI API key"));
    assert!(page.contains(Provider::OpenAi.keys_url()));
    assert!(!page.contains("platform.claude.com"));
}

#[tokio::test]
async fn the_model_list_follows_the_provider() {
    let (_fake, demo) = setup().await;
    let (router, cookie, visitor) = page_visitor(&demo);
    visitor.set_provider(Provider::OpenAi);
    visitor.set_key(OPENAI_KEY).unwrap();

    let (_, page) = get(&router, "/full/live", Some(&cookie)).await;
    for model in Model::for_provider(Provider::OpenAi) {
        let option = format!("<option value=\"{}\"", model.id());
        assert!(page.contains(&option), "missing {option}");
    }
    for model in Model::for_provider(Provider::Anthropic) {
        assert!(!page.contains(model.id()), "shows {}", model.id());
    }
    assert!(page.contains("Billed to your OpenAI account"));
}

#[tokio::test]
async fn the_key_never_appears_in_pages_or_printed_state() {
    key_stays_out_of_pages(Provider::Anthropic, KEY_A, "visitor-a").await;
    key_stays_out_of_pages(Provider::OpenAi, OPENAI_KEY, "visitor-c").await;
}

/// Has a conversation with a reply, a tool call and an error in it, then checks that
/// no page and nothing printable shows `key` or the distinctive `part` of it.
async fn key_stays_out_of_pages(provider: Provider, key: &str, part: &str) {
    let (_fake, demo) = setup().await;
    let router = site(&demo);
    let token = Token::random();
    let cookie = format!("__Host-demo={}", token.encode());
    let visitor = demo.visitor(&token.hash()).unwrap();
    visitor.set_provider(provider);
    visitor.set_key(key).unwrap();

    assert!(visitor.send(&demo, "What is 3 * 3?", ip(13)));
    settle(&visitor).await;
    assert!(visitor.send(&demo, "[401] and this?", ip(13)));
    settle(&visitor).await;

    for path in ["/full/live", "/blocks", "/full/chat-app", "/"] {
        let (response, body) = get(&router, path, Some(&cookie)).await;
        assert_eq!(response.status(), 200, "{path}");
        assert!(!body.contains(key), "{path} shows the key");
        assert!(!body.contains(part), "{path} shows part of the key");
        for value in response.headers().values() {
            assert!(!value.to_str().unwrap_or_default().contains(key));
        }
    }
    let (_, page) = get(&router, "/full/live", Some(&cookie)).await;
    assert!(page.contains("Clear key"), "the page shows the key is set");
    assert!(page.contains("The API key was rejected"));
    assert!(page.contains("calculate"));

    // Everything the server could print about the visitor's state.
    let snapshot = visitor.snapshot();
    let mut printed = Vec::new();
    for turn in &snapshot.turns {
        for reply in &turn.replies {
            printed.push(format!(
                "{:?} {:?} {:?}",
                reply.state, reply.parts, reply.notice
            ));
        }
    }
    printed.push(format!(
        "{:?}",
        super::key::ApiKey::parse(provider, key).unwrap()
    ));
    for text in printed {
        assert!(!text.contains(key), "{text}");
    }
}

// The same behavior against OpenAI's Responses API, through the fake's OpenAI mode.

/// A new visitor on `provider` with `key` set.
fn visitor_with(demo: &Demo, provider: Provider, key: &str) -> Arc<Visitor> {
    let visitor = demo.visitor(&session()).unwrap();
    visitor.set_provider(provider);
    visitor.set_key(key).unwrap();
    visitor
}

fn kinds(reply: &ReplyView) -> Vec<&'static str> {
    reply
        .parts
        .iter()
        .map(|part| match part {
            Part::Thinking { .. } => "thinking",
            Part::Tool { .. } => "tool",
            Part::Text(_) => "text",
        })
        .collect()
}

/// The `input` items of the last request the fake received.
fn last_input(fake: &FakeApi) -> Vec<Value> {
    fake.requests().last().unwrap().body["input"]
        .as_array()
        .unwrap()
        .clone()
}

#[tokio::test]
async fn openai_streams_reasoning_a_tool_call_and_text() {
    let (fake, demo) = setup().await;
    let visitor = visitor_with(&demo, Provider::OpenAi, OPENAI_KEY);
    assert!(visitor.send(&demo, "what is 2+2", ip(40)));

    let reply = last_reply(&settle(&visitor).await);
    assert_eq!(reply.state, ReplyState::Done, "{:?}", reply.notice);
    assert!(reply.notice.is_none());
    assert_eq!(
        kinds(&reply),
        ["thinking", "text", "tool", "thinking", "text"]
    );
    let Part::Tool {
        name,
        status,
        input,
        output,
        ..
    } = &reply.parts[2]
    else {
        unreachable!()
    };
    assert_eq!(name, "calculate");
    assert_eq!(*status, ToolStatus::Completed);
    assert_eq!(
        serde_json::from_str::<Value>(input).unwrap(),
        json!({"expression": "2+2"})
    );
    assert!(output.as_deref().unwrap().contains('4'));
    let Part::Thinking {
        text, streaming, ..
    } = &reply.parts[0]
    else {
        unreachable!()
    };
    assert!(text.contains("calculate"));
    assert!(!streaming);

    let requests = fake.requests();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert_eq!(request.authorization, format!("Bearer {OPENAI_KEY}"));
        assert_eq!(request.api_key, "");
        assert_eq!(request.path, "/v1/responses");
        assert_eq!(request.body["model"], Provider::OpenAi.default_model().id());
        assert_eq!(request.body["store"], false);
        assert_eq!(request.body["stream"], true);
        assert_eq!(request.body["max_output_tokens"], MAX_TOKENS);
        assert_eq!(request.body["reasoning"]["summary"], "auto");
    }
    let second = requests[1].body["input"].as_array().unwrap();
    let call = &second[second.len() - 2];
    let result = &second[second.len() - 1];
    assert_eq!(call["type"], "function_call");
    assert_eq!(result["type"], "function_call_output");
    assert_eq!(result["call_id"], call["call_id"]);
    assert_eq!(result["output"], "4");
    // The reasoning item goes back as it came, encrypted content included.
    assert_eq!(second[1]["type"], "reasoning");
    assert!(second[1]["encrypted_content"].is_string());

    // The conversation continues from the full history.
    assert!(visitor.send(&demo, "[text] thanks", ip(40)));
    assert_eq!(last_reply(&settle(&visitor).await).state, ReplyState::Done);
    let third = last_input(&fake);
    assert_eq!(&third[..second.len()], second.as_slice());
}

#[tokio::test]
async fn openai_api_errors_render_as_notices() {
    let (_fake, demo) = setup().await;
    let cases = [
        ("[401]", "(401)", "rejected", true),
        ("[429]", "(429)", "rate limit", false),
        ("[500]", "(500)", "internal error", false),
        ("[503]", "(503)", "overloaded", false),
    ];
    for (marker, code, phrase, key_problem) in cases {
        let visitor = visitor_with(&demo, Provider::OpenAi, OPENAI_KEY);
        assert!(visitor.send(&demo, &format!("{marker} hello"), ip(41)));
        let reply = last_reply(&settle(&visitor).await);
        assert_eq!(reply.state, ReplyState::Failed, "{marker}");
        let notice = reply.notice.expect("a notice");
        assert!(notice.error);
        assert!(notice.text.contains(code), "{marker}: {}", notice.text);
        assert!(notice.text.contains(phrase), "{marker}: {}", notice.text);
        assert_eq!(notice.key_problem, key_problem, "{marker}");
        assert!(!notice.text.contains(OPENAI_KEY));
        assert_eq!(visitor.api_history().len(), 1);
    }
}

#[tokio::test]
async fn openai_no_credit_is_a_billing_notice() {
    let (_fake, demo) = setup().await;
    let visitor = visitor_with(&demo, Provider::OpenAi, OPENAI_KEY);
    assert!(visitor.send(&demo, "[402] hello", ip(42)));
    let reply = last_reply(&settle(&visitor).await);
    let notice = reply.notice.expect("a notice");
    assert!(notice.error);
    assert!(notice.text.contains("(429)"), "{}", notice.text);
    assert!(
        notice
            .text
            .contains("Your OpenAI account has a billing problem"),
        "{}",
        notice.text
    );
    assert!(!notice.text.contains("rate limit"), "{}", notice.text);
    assert!(!notice.key_problem);
}

#[tokio::test]
async fn openai_without_reasoning_shows_no_thinking_part() {
    let (_fake, demo) = setup().await;
    let visitor = visitor_with(&demo, Provider::OpenAi, OPENAI_KEY);
    assert!(visitor.send(&demo, "[nothink] hi", ip(43)));
    let reply = last_reply(&settle(&visitor).await);
    assert_eq!(reply.state, ReplyState::Done, "{:?}", reply.notice);
    assert_eq!(kinds(&reply), ["text"]);
    assert!(!reply.text().is_empty());
}

#[tokio::test]
async fn openai_invalid_tool_arguments_become_a_tool_error() {
    let (fake, demo) = setup().await;
    let visitor = visitor_with(&demo, Provider::OpenAi, OPENAI_KEY);
    assert!(visitor.send(&demo, "[badargs] 1+", ip(44)));
    let reply = last_reply(&settle(&visitor).await);
    assert_eq!(reply.state, ReplyState::Done, "{:?}", reply.notice);
    let Some(Part::Tool { status, error, .. }) = reply
        .parts
        .iter()
        .find(|part| matches!(part, Part::Tool { .. }))
    else {
        panic!("a tool part");
    };
    assert_eq!(*status, ToolStatus::Error);
    assert!(error);
    let input = last_input(&fake);
    let output = input
        .iter()
        .find(|item| item["type"] == "function_call_output")
        .expect("a function_call_output");
    assert!(
        output["output"]
            .as_str()
            .unwrap()
            .starts_with("INVALID_JSON"),
        "{output}"
    );
}

#[tokio::test]
async fn openai_error_text_never_contains_the_key() {
    let (_fake, demo) = setup().await;
    let visitor = visitor_with(&demo, Provider::OpenAi, OPENAI_KEY);
    assert!(visitor.send(&demo, "[echokey] hello", ip(45)));
    let reply = last_reply(&settle(&visitor).await);
    let notice = reply.notice.expect("a notice");
    assert!(notice.key_problem);
    assert!(notice.text.contains("(401)"), "{}", notice.text);
    let tail = &OPENAI_KEY[OPENAI_KEY.len() - 20..];
    assert!(!notice.text.contains(OPENAI_KEY), "{}", notice.text);
    assert!(!notice.text.contains(tail), "{}", notice.text);
}

#[tokio::test]
async fn openai_stop_mid_stream_keeps_text() {
    let (fake, demo) = setup().await;
    let visitor = visitor_with(&demo, Provider::OpenAi, OPENAI_KEY);
    assert!(visitor.send(&demo, "[slow] go", ip(46)));
    wait_for(&visitor, |snapshot| {
        last_reply(snapshot).text().split_whitespace().count() >= 6
    })
    .await;
    visitor.stop();

    let reply = last_reply(&visitor.snapshot());
    assert_eq!(reply.state, ReplyState::Stopped);
    let text = reply.text();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(last_reply(&visitor.snapshot()).text(), text);
    assert!(text.len() < "This reply streams slowly so there is time to stop it. ".len() * 4);

    // The fake rejects malformed input with a 400, so a Done reply shows the
    // interrupted text went back as a well-formed item.
    assert!(visitor.send(&demo, "[text] and now?", ip(46)));
    let next = last_reply(&settle(&visitor).await);
    assert_eq!(next.state, ReplyState::Done, "{:?}", next.notice);
    let input = last_input(&fake);
    assert_eq!(
        input[1],
        json!({"type": "message", "role": "assistant", "content": text})
    );
}

#[tokio::test]
async fn openai_stop_during_tool_call_cancels_it() {
    let (fake, demo) = setup().await;
    let visitor = visitor_with(&demo, Provider::OpenAi, OPENAI_KEY);
    assert!(visitor.send(&demo, "2 + 2", ip(47)));
    wait_for(&visitor, |snapshot| {
        last_reply(snapshot)
            .parts
            .iter()
            .any(|part| matches!(part, Part::Tool { .. }))
    })
    .await;
    visitor.stop();
    let reply = last_reply(&visitor.snapshot());
    let Some(Part::Tool { status, .. }) = reply
        .parts
        .iter()
        .find(|part| matches!(part, Part::Tool { .. }))
    else {
        unreachable!()
    };
    assert_eq!(*status, ToolStatus::Error);

    assert!(visitor.send(&demo, "[text] ok", ip(47)));
    let next = last_reply(&settle(&visitor).await);
    assert_eq!(next.state, ReplyState::Done, "{:?}", next.notice);
    // No function_call without its output went back.
    assert!(
        !last_input(&fake)
            .iter()
            .any(|item| item["type"] == "function_call")
    );
}

#[tokio::test]
async fn openai_regenerate_adds_a_branch() {
    let (fake, demo) = setup().await;
    let visitor = visitor_with(&demo, Provider::OpenAi, OPENAI_KEY);
    assert!(visitor.send(&demo, "[text] one", ip(48)));
    settle(&visitor).await;
    assert!(visitor.regenerate(&demo, ip(48)));
    let snapshot = settle(&visitor).await;
    assert_eq!(snapshot.turns[0].replies.len(), 2);
    assert_eq!(snapshot.turns[0].current, 1);
    assert_eq!(
        last_reply(&snapshot).state,
        ReplyState::Done,
        "{:?}",
        last_reply(&snapshot).notice
    );
    // The regenerated request is the question alone.
    assert_eq!(
        last_input(&fake),
        [json!({"type": "message", "role": "user", "content": "[text] one"})]
    );

    assert!(visitor.select_branch(snapshot.turns[0].id, 0));
    assert_eq!(visitor.snapshot().turns[0].current, 0);
}

#[tokio::test]
async fn openai_midstream_error_keeps_text() {
    let (fake, demo) = setup().await;
    let visitor = visitor_with(&demo, Provider::OpenAi, OPENAI_KEY);
    assert!(visitor.send(&demo, "[midstream] go", ip(49)));
    let reply = last_reply(&settle(&visitor).await);
    assert_eq!(reply.state, ReplyState::Failed);
    let notice = reply.notice.clone().unwrap();
    assert!(notice.text.contains("overloaded"), "{}", notice.text);
    assert!(notice.text.contains("mid-stream"), "{}", notice.text);
    assert_eq!(reply.text(), "This reply starts, and then");

    assert!(visitor.send(&demo, "[text] again", ip(49)));
    let next = last_reply(&settle(&visitor).await);
    assert_eq!(next.state, ReplyState::Done, "{:?}", next.notice);
    assert_eq!(
        last_input(&fake)[1],
        json!({"type": "message", "role": "assistant", "content": "This reply starts, and then"})
    );
}
