use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::loader::{LoaderSize, LoaderVariant, loader};

#[component]
pub async fn example(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="flex flex-col gap-6">
            for variant in [
                LoaderVariant::Dots,
                LoaderVariant::Wave,
                LoaderVariant::Circular,
            ] {
                <div class="flex items-center gap-8">
                    for size in [LoaderSize::Sm, LoaderSize::Md, LoaderSize::Lg] {
                        loader(variant: variant, size: size)
                    }
                </div>
            }
        </div>
    })
}
