//! `/full/...`: each block on a page of its own, filling the window, and the
//! real-model demo.

pub mod slug;

use topcoat::{
    Result,
    context::Cx,
    router::{Body, Next, layer, page, response::Response},
    session,
    view::{View, view},
};

/// Gives each visitor of the real-model demo a session before the page renders, so
/// the cookie is set by the page's own response and the page's live connection,
/// opened after it, carries it.
#[layer("./claude")]
async fn claude_session(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    if session::token_hash(cx).await?.is_none() {
        session::start(cx).await?;
    }
    next.run(cx, body).await
}

/// The chat demo answered by the real Claude API, on the visitor's own key.
#[page("./claude")]
async fn claude(cx: &Cx) -> Result<impl View> {
    Ok(view! { crate::claude::view::page() })
}
