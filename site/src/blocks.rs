//! `/blocks`: larger examples composed from several components.

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::page,
    view::{View, view},
};

use crate::{
    catalog::BLOCKS,
    shell::{example_section, page_header, pager},
};

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        page_header(
            title: "Blocks",
            description: "Larger examples built from top-prompt and Topcoat components. Copy \
                one into your app as a starting point."
        )
        <div class="mt-12 flex flex-col gap-14">
            #[key(block.slug)]
            for block in BLOCKS {
                <div class="relative">
                    <a
                        href=(format!("/full/{}", block.slug))
                        class="absolute top-[40px] right-0 z-10 inline-flex h-7 items-center gap-1.5 rounded-md px-2 text-xs text-muted-foreground transition-colors outline-none hover:bg-foreground/5 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
                    >
                        "Open full page"
                        icon(data: iconify_icon!("lucide:arrow-up-right"))
                    </a>
                    example_section(example: &block.example, wide: true)
                </div>
            }
        </div>
        pager(path: "/blocks")
    })
}
