//! Product — photograph, the price, the spec table, and the desk's note.

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::catalog::{self, Product};
use crate::ui::{Kicker, Photo};

/// The product addressed by `/p/:id`, if there is one.
pub fn current_product() -> Option<&'static Product> {
    let params = use_params_map();
    params.read().get("id").and_then(|id| catalog::find(&id))
}

#[component]
pub fn ProductScreen() -> impl IntoView {
    let params = use_params_map();
    let product = move || params.read().get("id").and_then(|id| catalog::find(&id));

    move || match product() {
        Some(p) => view! { <ProductBody product=p /> }.into_any(),
        None => view! {
            <div class="px-[18px] py-12">
                <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">
                    "That object is not in this issue."
                </div>
                <p class="mt-2 mb-4 text-[13px] leading-[1.6] text-ink/62">
                    "It may have sold out and left the index."
                </p>
                <A href="/" {..} class="btn btn-primary no-underline">
                    "Back to Issue 14"
                </A>
            </div>
        }
        .into_any(),
    }
}

#[component]
fn ProductBody(product: &'static Product) -> impl IntoView {
    view! {
        <div>
            <div class="h-[300px]">
                <Photo label="Product photo" />
            </div>

            <div class="border-b-2 border-ink/40 px-[18px] pt-[18px] pb-4">
                <Kicker class="text-accent-700">{product.cat} " · " {product.num}</Kicker>
                <h2 class="mt-2 text-[30px] leading-[1.05] tracking-[-0.025em]">{product.name}</h2>
                <div class="mt-2.5 flex items-baseline gap-3">
                    <div class="font-heading text-[22px] font-extrabold">
                        {product.price_label()}
                    </div>
                    <div class="text-xs text-ink/55">{product.stock}</div>
                </div>
                <p class="mt-3.5 text-[13.5px] leading-[1.65] text-pretty">{product.blurb}</p>
            </div>

            <div class="border-b border-ink/18 px-[18px] py-4">
                <Kicker class="mb-2.5">"Specification"</Kicker>
                {product
                    .specs
                    .iter()
                    .map(|s| {
                        view! {
                            <div class="grid grid-cols-[1fr_auto] gap-4 border-t border-ink/14 py-[9px] text-[12.5px]">
                                <div class="text-ink/58">{s.k}</div>
                                <div class="text-right font-semibold">{s.v}</div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>

            <div class="border-b-2 border-ink/40 bg-surface p-[18px]">
                <Kicker class="text-accent-700">"From Issue 14"</Kicker>
                <p class="mt-2.5 text-sm leading-[1.6] text-pretty">{product.note}</p>
                <div class="mt-3 text-[11.5px] uppercase tracking-[0.06em] text-ink/55">
                    "Written by the buying desk"
                </div>
            </div>
            <div class="h-3"></div>
        </div>
    }
}
