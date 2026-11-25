//! `/full/...`: each block on a page of its own, filling the window, and the
//! real-model demo.

pub mod slug;

use topcoat::{
    Result,
    context::Cx,
    router::{Body, Next, error::redirect_permanent, layer, page, response::Response, route},
    session,
    view::{View, view},
};

/// Gives each visitor of the real-model demo a session before the page renders, so
/// the cookie is set by the page's own response and the page's live connection,
/// opened after it, carries it.
#[layer("./live")]
async fn live_session(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    if session::token_hash(cx).await?.is_none() {
        session::start(cx).await?;
    }
    next.run(cx, body).await
}

/// The chat demo answered by a real model, Anthropic's or OpenAI's, on the visitor's
/// own key.
#[page("./live")]
async fn live(cx: &Cx) -> Result<impl View> {
    Ok(view! { crate::live::view::page() })
}

/// The demo's old address, from before it took more than one provider.
#[route(GET "./claude")]
async fn claude(cx: &Cx) -> Result<()> {
    Err(redirect_permanent("/full/live").into())
}
