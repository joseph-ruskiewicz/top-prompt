use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::loader::{LoaderVariant, loader};

const VARIANTS: [(LoaderVariant, &str); 8] = [
    (LoaderVariant::Dots, "Dots"),
    (LoaderVariant::Typing, "Typing"),
    (LoaderVariant::Pulse, "Pulse"),
    (LoaderVariant::Circular, "Circular"),
    (LoaderVariant::Wave, "Wave"),
    (LoaderVariant::Bars, "Bars"),
    (LoaderVariant::Text, "Text"),
    (LoaderVariant::TextShimmer, "Text shimmer"),
];

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="grid w-full grid-cols-2 gap-6 sm:grid-cols-4">
            for (variant, name) in VARIANTS {
                <div class="flex flex-col items-center gap-3">
                    <div class="flex h-8 items-center">loader(variant: variant)</div>
                    <span class="text-xs text-muted-foreground">(name)</span>
                </div>
            }
        </div>
    })
}
