//! The cookie that identifies a visitor's session.
//!
//! Topcoat's default session cookie is `SameSite=Lax` and lasts thirty days. The demo
//! wants less: `SameSite=Strict`, so no other site can make a request that carries
//! it, and no `Max-Age`, so the browser drops it when it closes. The cookie holds a
//! random token; the server keeps only the token's hash, and the API key never goes
//! in a cookie.

use std::time::Duration;

use topcoat::{
    context::Cx,
    cookie::{Cookie, Cookies, SameSite, cookies},
    session::{Token, TokenStore, TokenStoreFuture},
};

/// The cookie name. The `__Host-` prefix, added on write, also requires `Secure` and
/// `Path=/` and forbids a `Domain`.
const NAME: &str = "demo";

/// A [`TokenStore`] for `__Host-demo`: `Secure`, `HttpOnly`, `SameSite=Strict`, for
/// the browser session only.
pub struct StrictSessionCookie;

fn hardened(cx: &Cx) -> impl Cookies {
    cookies(cx)
        .override_same_site(SameSite::Strict)
        .override_http_only(true)
        .override_secure(true)
        .override_path("/")
        .override_prefix_host()
}

impl TokenStore for StrictSessionCookie {
    fn read<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>> {
        Box::pin(async move {
            Ok(hardened(cx)
                .get(NAME)
                .and_then(|cookie| Token::decode(cookie.value_trimmed()).ok()))
        })
    }

    fn write<'a>(
        &'a self,
        cx: &'a Cx,
        token: Token,
        _max_age: Duration,
    ) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move {
            hardened(cx).add(Cookie::new(NAME, token.encode()));
            Ok(())
        })
    }

    fn delete<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move {
            hardened(cx).remove(Cookie::new(NAME, ""));
            Ok(())
        })
    }
}
