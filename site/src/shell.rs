//! The docs shell: the sidebar, the content column, and the parts every docs page is
//! made of. These are site components, not registry ones.

use topcoat::{
    Result,
    context::Cx,
    cookie::{Cookie, Cookies, SameSite, cookies, time::Duration},
    icon::{icon, iconify::iconify_icon},
    runtime::{Event, Signal, link, procedure, signal},
    view::{Attributes, Child, StaticClass, Unescaped, View, attributes, class, component, view},
};

use crate::{
    GITHUB_URL, api,
    catalog::{self, ComponentDoc, Dependency, Example},
    components::{
        button::{ButtonSize, ButtonVariant, button},
        code_block::{code_block, code_block_copy},
        sheet::{SheetSide, sheet, sheet_content},
    },
    highlight,
};

/// The name of the cookie that remembers the theme.
const THEME_COOKIE: &str = "theme";

/// Whether the request's theme cookie asks for the dark theme. The layout reads it so
/// the first HTML already has the right class, with no flash of the other theme.
pub fn dark_from_cookie(cx: &Cx) -> bool {
    cookies(cx)
        .get(THEME_COOKIE)
        .is_some_and(|cookie| cookie.value() == "dark")
}

/// Remembers the reader's theme for a year. The procedure's response carries the
/// `Set-Cookie` header, so no navigation is needed.
#[procedure]
async fn save_theme(cx: &Cx, dark: bool) -> Result<()> {
    cookies(cx).add(
        Cookie::build((THEME_COOKIE, if dark { "dark" } else { "light" }))
            .path("/")
            .max_age(Duration::days(365))
            .same_site(SameSite::Lax)
            .http_only(true),
    );
    Ok(())
}

/// The sidebar and the content column around a page.
#[component]
pub async fn shell(
    cx: &Cx,
    path: &str,
    dark: Signal<bool>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let menu = signal(cx, || false);
    let wide = path == "/blocks";
    let sheet_dark = dark.clone();

    Ok(view! {
        // Narrow screens: a bar with the menu button; the sidebar is a sheet.
        <header
            class="sticky top-0 z-30 flex h-14 items-center gap-2 border-b border-dashed border-border bg-background px-3 lg:hidden"
        >
            button(
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Icon,
                attrs: attributes! {
                    type="button"
                    aria-label="Open menu"
                    aria-haspopup="dialog"
                    @click=$(|_e: Event| menu.set(true))
                },
                icon(data: iconify_icon!("lucide:menu"))
            )
            logo()
        </header>
        sheet(
            open: $(menu.get()),
            attrs: attributes! { aria-label="Menu" },
            sheet_content(
                side: SheetSide::Left,
                attrs: attributes! {
                    class="w-72 gap-0 p-0"
                    @click=$(|_e: Event| {
                        // Following a link closes the menu; the next page renders with it
                        // closed.
                        if raw!(
                            "cx.hydrate(${_e}.inner.target.closest('a') !== null)",
                            false,
                        ) {
                            menu.set(false);
                        }
                    })
                },
                <div class="flex items-center justify-between px-5 pt-5 pb-4">
                    logo()
                    button(
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Icon,
                        attrs: attributes! {
                            type="button"
                            aria-label="Close menu"
                            @click=$(|_e: Event| menu.set(false))
                        },
                        icon(data: iconify_icon!("lucide:x"))
                    )
                </div>
                sidebar_body(path: path, dark: sheet_dark)
            )
        )

        // Wide screens: a fixed sidebar.
        <aside
            class="fixed inset-y-0 left-0 z-20 hidden w-60 flex-col border-r border-dashed border-border bg-background lg:flex"
        >
            <div class="px-5 pt-8 pb-16">logo()</div>
            sidebar_body(path: path, dark: dark)
        </aside>

        <div class="lg:pl-60">
            <main
                class=(class!(
                    "mx-auto w-full px-5 pt-10 pb-24 sm:px-8 lg:pt-32 xl:mx-0 xl:ml-60 xl:px-0",
                    if wide { "max-w-5xl" } else { "max-w-3xl" },
                ))
            >
                (child)
            </main>
        </div>
    })
}

/// The mark and the wordmark, linking home.
#[component]
async fn logo(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        link(
            href: "/",
            attrs: attributes! {
                class="flex w-fit items-center gap-2 rounded-md text-base font-medium outline-none focus-visible:ring-2 focus-visible:ring-ring"
            },
            <span
                aria-hidden="true"
                class="flex size-6 items-center justify-center rounded-md bg-foreground text-background"
            >
                icon(
                    data: iconify_icon!("lucide:chevron-right"),
                    attrs: attributes! { class="size-4" }
                )
            </span>
            "top-prompt"
        )
    })
}

/// The link groups and the theme toggle, shared by the sidebar and the menu sheet.
#[component]
async fn sidebar_body(cx: &Cx, path: &str, dark: Signal<bool>) -> Result<impl View> {
    Ok(view! {
        <nav
            aria-label="Docs"
            class="flex min-h-0 flex-1 flex-col gap-10 overflow-y-auto px-3 pb-6"
        >
            nav_group(
                label: "Get Started",
                nav_link(href: "/", current: path == "/", "Home")
                nav_link(
                    href: "/docs/introduction",
                    current: path == "/docs/introduction",
                    "Introduction"
                )
                nav_link(
                    href: "/docs/installation",
                    current: path == "/docs/installation",
                    "Installation"
                )
            )
            nav_group(
                label: "Components",
                for doc in catalog::COMPONENTS {
                    let href = format!("/docs/{}", doc.name);
                    nav_link(
                        href: &href,
                        current: path == href,
                        new: doc.new,
                        (doc.title)
                    )
                }
            )
            nav_group(
                label: "Blocks",
                nav_link(href: "/blocks", current: path == "/blocks", "Blocks")
            )
            nav_group(
                label: "Resources",
                nav_link(href: "/llms.txt", plain: true, "llms.txt")
                nav_link(href: GITHUB_URL, plain: true, "GitHub")
            )
        </nav>
        <div class="border-t border-dashed border-border px-3 py-3">
            let label = topcoat::runtime::expr!(
                if dark.get() {
                    "Switch to light theme"
                } else {
                    "Switch to dark theme"
                },
            );
            <button
                type="button"
                class=(NAV_LINK)
                @click=$(async |_e: Event| {
                    // The theme switches at once; the cookie is saved in the background.
                    dark.toggle();
                    save_theme(dark.get()).await;
                })
                :aria-label=(label)
            >
                <span class="contents" :hidden=$(!dark.get())>
                    icon(data: iconify_icon!("lucide:sun"))
                    "Light mode"
                </span>
                <span class="contents" :hidden=$(dark.get())>
                    icon(data: iconify_icon!("lucide:moon"))
                    "Dark mode"
                </span>
            </button>
        </div>
    })
}

/// A labelled group of sidebar links.
#[component]
async fn nav_group(cx: &Cx, label: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <div class="flex flex-col">
            <span class="px-2 pb-1.5 text-sm font-medium text-muted-foreground/80">
                (label)
            </span>
            <ul class="flex flex-col gap-0.5">(child)</ul>
        </div>
    })
}

/// Classes for a sidebar link or button.
const NAV_LINK: StaticClass = class!(
    "flex h-8 w-full items-center gap-2 rounded-lg px-2 text-left text-sm text-foreground/80",
    "outline-none transition-colors hover:bg-foreground/5 hover:text-foreground",
    "focus-visible:ring-2 focus-visible:ring-ring",
    "aria-[current=page]:bg-foreground/5 aria-[current=page]:font-medium",
    "aria-[current=page]:text-foreground",
);

/// A sidebar link. `plain` links are ordinary links, for files and other sites.
#[component]
async fn nav_link(
    cx: &Cx,
    href: &str,
    #[default] current: bool,
    #[default] new: bool,
    #[default] plain: bool,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <li>
            if plain {
                <a href=(href) class=(NAV_LINK)>(child)</a>
            } else {
                link(
                    href: href,
                    attrs: attributes! {
                        class=(NAV_LINK)
                        aria-current=(current.then_some("page"))
                    },
                    (child)
                    if new {
                        <span class="text-xs text-muted-foreground">"new"</span>
                    }
                )
            }
        </li>
    })
}

/// A page's title and one-line description.
#[component]
pub async fn page_header(cx: &Cx, title: &str, #[default] description: &str) -> Result<impl View> {
    Ok(view! {
        <header class="flex flex-col gap-3">
            <h1 class="text-2xl leading-8 font-semibold tracking-tight">(title)</h1>
            if !description.is_empty() {
                <p class="text-base text-muted-foreground">(description)</p>
            }
        </header>
    })
}

/// A section heading on a docs page.
#[component]
pub async fn section_heading(
    cx: &Cx,
    #[default] id: &str,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <h2
            id=((!id.is_empty()).then_some(id))
            class="mt-12 mb-5 scroll-mt-20 text-xl leading-7 font-medium tracking-tight"
        >
            (child)
        </h2>
    })
}

/// A bordered panel with a live example centered in it. A `wide` panel is for blocks:
/// a fixed height, no padding, and the example filling it.
#[component]
pub async fn preview_panel(
    cx: &Cx,
    #[default] wide: bool,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "flex w-full overflow-hidden rounded-lg border border-border",
                if wide {
                    "h-[650px]"
                } else {
                    "min-h-[350px] items-center justify-center p-6 sm:p-10"
                },
                attrs.remove("class"),
            ))
            (attrs)
        >
            <div
                class=(if wide {
                    "flex h-full w-full flex-col"
                } else {
                    "flex w-full flex-col items-center"
                })
            >
                (child)
            </div>
        </div>
    })
}

/// Rust source, highlighted, in the registry's code block, scrolling past a height.
#[component]
pub async fn code_panel(
    cx: &Cx,
    source: &str,
    #[default] language: Option<&str>,
) -> Result<impl View> {
    let language = language.unwrap_or("rust");
    let markup = (language == "rust").then(|| Unescaped::new_unchecked(highlight::rust(source)));

    Ok(view! {
        <div class="code-panel">
            code_block(
                language: Some(language),
                code: source,
                attrs: attributes! {
                    class="[&_pre]:max-h-[650px] [&_pre]:overflow-y-auto [&_pre]:text-[13px] [&_pre]:leading-[1.75]"
                },
                match markup {
                    Some(markup) => (markup),
                    None => (source),
                }
            )
        </div>
    })
}

/// A short command, such as `topcoat ui add`, with a copy button.
#[component]
pub async fn command(cx: &Cx, text: &str) -> Result<impl View> {
    Ok(view! {
        <div
            data-slot="command"
            class="flex items-center gap-2 rounded-lg border border-border py-1 pr-1 pl-4"
        >
            <code
                class="min-w-0 flex-1 overflow-x-auto py-2 font-mono text-[13px] whitespace-pre"
            >
                (text)
            </code>
            code_block_copy(
                text: text,
                attrs: attributes! { aria-label="Copy command" }
            )
        </div>
    })
}

/// Classes for a tab of a two-tab switcher.
const TAB: StaticClass = class!(
    "relative -mb-px inline-flex h-10 cursor-pointer items-center px-3 text-sm font-medium",
    "text-muted-foreground outline-none transition-colors hover:text-foreground",
    "focus-visible:ring-2 focus-visible:ring-ring",
    "aria-selected:text-foreground aria-selected:after:absolute aria-selected:after:inset-x-1",
    "aria-selected:after:bottom-0 aria-selected:after:h-0.5 aria-selected:after:rounded-full",
    "aria-selected:after:bg-foreground",
);

/// Two tabs, such as Preview and Code, over two panels. The first is selected until
/// the reader picks the second; without the runtime, only the first shows.
#[component]
pub async fn tab_pair(
    cx: &Cx,
    first: &str,
    second: &str,
    #[into] second_panel: Child<'_>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let showing_second = signal(cx, || false);
    let id = topcoat::view::element_id(cx);
    let (first_tab, second_tab) = (format!("{id}-tab-1"), format!("{id}-tab-2"));
    let (first_panel, second_panel_id) = (format!("{id}-panel-1"), format!("{id}-panel-2"));

    Ok(view! {
        <div class="flex flex-col gap-2">
            <div role="tablist" class="flex items-center gap-1 border-b border-border">
                <button
                    type="button"
                    role="tab"
                    id=(&first_tab)
                    aria-controls=(&first_panel)
                    class=(TAB)
                    :aria-selected=$(if showing_second.get() { "false" } else { "true" })
                    @click=$(|_e: Event| showing_second.set(false))
                >
                    (first)
                </button>
                <button
                    type="button"
                    role="tab"
                    id=(&second_tab)
                    aria-controls=(&second_panel_id)
                    class=(TAB)
                    :aria-selected=$(if showing_second.get() { "true" } else { "false" })
                    @click=$(|_e: Event| showing_second.set(true))
                >
                    (second)
                </button>
            </div>
            <div
                role="tabpanel"
                id=(&first_panel)
                aria-labelledby=(&first_tab)
                :hidden=$(showing_second.get())
            >
                (child)
            </div>
            <div
                role="tabpanel"
                id=(&second_panel_id)
                aria-labelledby=(&second_tab)
                :hidden=$(!showing_second.get())
            >
                (second_panel)
            </div>
        </div>
    })
}

/// An example: its heading, a sentence, and Preview and Code tabs.
#[component]
pub async fn example_section(
    cx: &Cx,
    example: &'static Example,
    #[default] wide: bool,
) -> Result<impl View> {
    Ok(view! {
        <section
            id=(example.id)
            class="flex scroll-mt-20 flex-col gap-2.5 not-first:mt-10"
        >
            <h3 class="text-base font-medium">(example.title)</h3>
            if !example.description.is_empty() {
                <p class="text-sm leading-relaxed text-muted-foreground">
                    (inline_code(example.description))
                </p>
            }
            tab_pair(
                first: "Preview",
                second: "Code",
                second_panel: topcoat::view::Child::new(
                    view! { code_panel(source: example.source) },
                ),
                preview_panel(wide: wide, ((example.render)(cx)))
            )
        </section>
    })
}

/// Turns `` `code` `` spans in a sentence into `<code>` elements.
fn inline_code(text: &str) -> Unescaped<String> {
    let mut out = String::new();
    for (index, part) in text.split('`').enumerate() {
        let escaped = part
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        if index % 2 == 1 {
            out.push_str(
                "<code class=\"rounded bg-foreground/5 px-1 py-0.5 font-mono text-[0.85em]\">",
            );
            out.push_str(&escaped);
            out.push_str("</code>");
        } else {
            out.push_str(&escaped);
        }
    }
    Unescaped::new_unchecked(out)
}

/// The Installation section of a component page: CLI and Manual tabs.
#[component]
pub async fn install_section(cx: &Cx, doc: &'static ComponentDoc) -> Result<impl View> {
    let add = format!("topcoat ui add {} --registry top-prompt", doc.name);
    let mut built_ins = Vec::new();
    let mut locals = Vec::new();
    for dependency in catalog::dependencies(doc.name) {
        match dependency {
            Dependency::BuiltIn(name) => built_ins.push(name),
            Dependency::Local(name) => locals.push(name),
        }
    }
    let built_ins = if built_ins.is_empty() {
        String::new()
    } else {
        format!("topcoat ui add {}", built_ins.join(" "))
    };

    Ok(view! {
        section_heading(id: "installation", "Installation")
        tab_pair(
            first: "CLI",
            second: "Manual",
            second_panel: topcoat::view::Child::new(
                view! {
                    <ol
                        class="list-decimal space-y-6 pl-5 text-sm leading-relaxed marker:text-muted-foreground"
                    >
                        if !doc.crates.is_empty() {
                            <li class="space-y-3 pl-1">
                                <p>"Add the crates it uses:"</p>
                                #[key(krate)]
                                for krate in doc.crates {
                                    let text = format!("cargo add {krate}");
                                    command(text: &text)
                                }
                            </li>
                        }
                        if !built_ins.is_empty() {
                            <li class="space-y-3 pl-1">
                                <p>"Install the built-in components it uses:"</p>
                                command(text: &built_ins)
                            </li>
                        }
                        if !locals.is_empty() {
                            <li class="space-y-3 pl-1">
                                <p>
                                    "Install the top-prompt components it uses, the same way:"
                                </p>
                                <ul class="flex flex-wrap gap-2">
                                    for name in &locals {
                                        let href = format!("/docs/{name}");
                                        <li>
                                            link(
                                                href: &href,
                                                attrs: attributes! {
                                                    class="rounded-md bg-foreground/5 px-2 py-1 font-mono text-xs underline-offset-4 hover:underline"
                                                },
                                                (name)
                                            )
                                        </li>
                                    }
                                </ul>
                            </li>
                        }
                        <li class="space-y-3 pl-1">
                            <p>
                                "Copy this source into "
                                <code
                                    class="rounded bg-foreground/5 px-1 py-0.5 font-mono text-[0.85em]"
                                >
                                    (format!("src/components/{}.rs", doc.name))
                                </code>
                                " and declare the module in "
                                <code
                                    class="rounded bg-foreground/5 px-1 py-0.5 font-mono text-[0.85em]"
                                >
                                    "src/components.rs"
                                </code>
                                ":"
                            </p>
                            code_panel(source: doc.source)
                        </li>
                    </ol>
                },
            ),
            command(text: &add)
        )
    })
}

/// The Component API section: a table per public component function, read from the
/// registry source.
#[component]
pub async fn api_section(cx: &Cx, doc: &'static ComponentDoc) -> Result<impl View> {
    let functions = api::functions(doc.source);

    Ok(view! {
        section_heading(id: "api", "Component API")
        <div class="flex flex-col gap-8">
            for function in functions {
                <div class="flex flex-col gap-2.5">
                    <h3 class="font-mono text-sm font-medium">(function.name)</h3>
                    <div class="overflow-x-auto rounded-lg border border-border">
                        <table class="w-full text-left text-sm">
                            <thead class="bg-foreground/[0.03]">
                                <tr class="border-b border-border">
                                    <th scope="col" class="h-10 px-4 font-medium">"Prop"</th>
                                    <th scope="col" class="h-10 px-4 font-medium">"Type"</th>
                                    <th scope="col" class="h-10 px-4 font-medium">
                                        "Default"
                                    </th>
                                    <th scope="col" class="h-10 px-4 font-medium">
                                        "Description"
                                    </th>
                                </tr>
                            </thead>
                            <tbody class="divide-y divide-border">
                                for prop in function.props {
                                    <tr>
                                        <td
                                            class="px-4 py-2.5 align-top font-mono text-[13px] whitespace-nowrap"
                                        >
                                            (prop.name)
                                        </td>
                                        <td
                                            class="px-4 py-2.5 align-top font-mono text-[13px] whitespace-nowrap text-muted-foreground"
                                        >
                                            (prop.ty)
                                        </td>
                                        <td
                                            class="px-4 py-2.5 align-top font-mono text-[13px] whitespace-nowrap text-muted-foreground"
                                        >
                                            match prop.default {
                                                None => <span class="font-sans text-xs">"required"</span>,
                                                Some(default) => (default),
                                            }
                                        </td>
                                        <td
                                            class="min-w-56 px-4 py-2.5 align-top text-muted-foreground"
                                        >
                                            (inline_code(&prop.description))
                                        </td>
                                    </tr>
                                }
                            </tbody>
                        </table>
                    </div>
                </div>
            }
        </div>
    })
}

/// Previous and next links, in sidebar order.
#[component]
pub async fn pager(cx: &Cx, path: &str) -> Result<impl View> {
    let pages = catalog::pages();
    let index = pages.iter().position(|page| page.href == path);
    let mut pages = pages.into_iter().map(Some).collect::<Vec<_>>();
    let mut take = |index: Option<usize>| index.and_then(|index| pages.get_mut(index)?.take());
    let previous = take(index.and_then(|index| index.checked_sub(1)));
    let next = take(index.map(|index| index + 1));

    Ok(view! {
        <nav
            aria-label="Pages"
            class="mt-16 flex items-center justify-between gap-4 text-sm"
        >
            match previous {
                Some(page) => {
                    link(
                        href: &page.href,
                        attrs: attributes! { class=(PAGER_LINK) },
                        icon(data: iconify_icon!("lucide:arrow-left"))
                        (page.title)
                    )
                }
                None => <span></span>,
            }
            match next {
                Some(page) => {
                    link(
                        href: &page.href,
                        attrs: attributes! { class=(PAGER_LINK) },
                        (page.title)
                        icon(data: iconify_icon!("lucide:arrow-right"))
                    )
                }
                None => <span></span>,
            }
        </nav>
    })
}

const PAGER_LINK: StaticClass = class!(
    "inline-flex items-center gap-2 rounded-lg px-3 py-2 font-medium text-foreground/80",
    "outline-none transition-colors hover:bg-foreground/5 hover:text-foreground",
    "focus-visible:ring-2 focus-visible:ring-ring",
);

/// Prose for the Introduction and Installation pages.
#[component]
pub async fn prose(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "mt-8 text-[15px] leading-7 text-foreground/90",
                "[&_p]:my-4 [&_h2]:mt-12 [&_h2]:mb-4 [&_h2]:text-xl [&_h2]:font-medium",
                "[&_h2]:tracking-tight [&_h2]:text-foreground [&_ul]:my-4 [&_ul]:list-disc",
                "[&_ul]:pl-5 [&_li]:my-1.5 [&_a]:font-medium [&_a]:text-foreground",
                "[&_a]:underline [&_a]:underline-offset-4",
                "[&_:not(pre)>code]:rounded [&_:not(pre)>code]:bg-foreground/5",
                "[&_:not(pre)>code]:px-1 [&_:not(pre)>code]:py-0.5",
                "[&_:not(pre)>code]:font-mono [&_:not(pre)>code]:text-[0.85em]",
                "[&_.code-panel]:my-5 [&_[data-slot=command]]:my-4",
            ))
        >
            (child)
        </div>
    })
}
