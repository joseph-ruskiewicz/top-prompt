//! The topcoat-prompt docs site, built with Topcoat and the topcoat-prompt components.

// Installed with `topcoat ui add --all --registry topcoat-prompt`, which also pulled in
// the built-in components they use, plus the built-in sheet for the mobile menu. Like
// any vendored library, not every item is used by this app.
#[allow(dead_code)]
mod components;

mod api;
mod blocks;
mod catalog;
mod docs;
mod examples;
mod full;
mod live;
mod shell;

use components::button::{ButtonSize, ButtonVariant, button_variants};
use topcoat::{
    Result,
    asset::{AssetBundle, AssetConfig, RouterBuilderAssetExt},
    context::Cx,
    cookie::CookieLayer,
    font::fontsource::fontsource_font,
    icon::{icon, iconify::iconify_icon},
    router::{
        ForwardedHeader, Router, RouterBuilderDiscoverExt, Slot, TrustedProxies, href, layout,
        module_router, page, request::uri, route,
    },
    runtime::{RuntimeLayer, link, signal},
    session::SessionTokenStore,
    tailwind,
    view::{View, attributes, view},
};

use crate::{
    examples::chat_app::Chat,
    live::{
        api::{Endpoint, Endpoints},
        chat::Demo,
        cookie::StrictSessionCookie,
        provider::Provider,
    },
};

/// Where the GitHub links point. A placeholder until the repository is public.
pub const GITHUB_URL: &str = "https://github.com/joseph-ruskiewicz/topcoat-prompt";

#[tokio::main]
async fn main() {
    let demo = Demo::new(endpoints().await);
    topcoat::start(router(demo, AssetBundle::load().unwrap()))
        .await
        .unwrap();
}

/// Where the real-model demo sends requests: each provider's production API. A
/// debug build started with `DEMO_FAKE_API=1` answers from a local fake instead, and
/// one started with `DEMO_ANTHROPIC_BASE_URL` or `DEMO_OPENAI_BASE_URL` uses that URL
/// for that provider; release builds ignore all three.
async fn endpoints() -> Endpoints {
    #[cfg(debug_assertions)]
    if std::env::var_os("DEMO_FAKE_API").is_some() {
        let fake = live::fake::FakeApi::start().await.unwrap();
        return Endpoints {
            anthropic: Endpoint::local(fake.url()),
            openai: Endpoint::local(fake.url()),
        };
    }
    Endpoints {
        anthropic: Endpoint::from_env(Provider::Anthropic),
        openai: Endpoint::from_env(Provider::OpenAi),
    }
}

/// The application's routes and services.
fn router(demo: Demo, assets: impl Into<AssetConfig>) -> Router {
    let mut builder = module_router!()
        .assets(assets)
        .with(Chat::default())
        .with(demo)
        .with(SessionTokenStore::new(StrictSessionCookie))
        .discover()
        .layer(RuntimeLayer::new())
        .layer(CookieLayer::new());
    // On Fly.io every request comes through Fly's proxy, which puts the client's
    // address in Fly-Client-IP; the demo's per-address limit reads it from there.
    if std::env::var_os("FLY_APP_NAME").is_some() {
        builder = builder.trusted_proxies(
            TrustedProxies::new()
                .nearest(1)
                .header(ForwardedHeader::Single("fly-client-ip".parse().unwrap())),
        );
    }
    builder.build()
}

/// The document. Pages under `/full/` fill the window; every other page sits in the
/// docs shell with the sidebar.
#[layout]
async fn layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    // Runtime links keep signals the pages share, so the theme survives navigation. A
    // full load starts from the theme cookie, light when there is none.
    let saved = shell::dark_from_cookie(cx);
    let dark = signal(cx, || saved);
    let path = uri(cx).path().to_owned();
    let full = path.starts_with("/full/");

    Ok(view! {
        <!DOCTYPE html>
        <html
            lang="en"
            :class=$(if dark.get() { "dark" } else { "" })
            :style=$(if dark.get() {
                "color-scheme: dark"
            } else {
                "color-scheme: light"
            })
        >
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"topcoat-prompt"</title>
                <meta
                    name="description"
                    content="Chat interface components for Topcoat apps, written in Rust."
                >
                topcoat::dev::script()
                topcoat::runtime::script()
                topcoat::font::link(font: fontsource_font!(GEIST, host: Asset))
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            // The body's colors and font come from the theme's base layer.
            <body class="antialiased">
                if full {
                    <main class="flex h-dvh flex-col">(slot)</main>
                } else {
                    shell::shell(path: &path, dark: dark, (slot))
                }
            </body>
        </html>
    })
}

/// The home page: what topcoat-prompt is, and a live composer with its code.
#[page]
async fn home(cx: &Cx) -> Result<impl View> {
    let example = &catalog::HOME_EXAMPLE;

    Ok(view! {
        <div class="flex flex-col gap-10">
            <div class="flex flex-col gap-5">
                <h1 class="text-3xl leading-9 font-[450] tracking-tight text-balance">
                    "Chat interface components for Topcoat."
                    <br>
                    <span class="text-muted-foreground">
                        "Accessible, server-rendered building blocks for AI apps in Rust."
                    </span>
                </h1>
                <div class="flex flex-wrap items-center gap-3">
                    link(
                        href: href!(docs::slug::page, docs::slug: "introduction".to_owned()),
                        attrs: attributes! {
                            class=(button_variants(
                                ButtonVariant::Primary,
                                ButtonSize::Md,
                            ))
                        },
                        "Get Started"
                    )
                    <a
                        href=(GITHUB_URL)
                        class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))
                    >
                        icon(data: iconify_icon!("lucide:github"))
                        "GitHub"
                    </a>
                </div>
            </div>
            shell::preview_panel(((example.render)(cx)))
            shell::code_panel(source: example.source)
            shell::pager(path: "/")
        </div>
    })
}

/// A plain-text index of the docs for language models, generated from the catalog.
#[route(GET "./llms.txt")]
async fn llms_txt(cx: &Cx) -> Result<String> {
    Ok(catalog::llms_txt())
}
