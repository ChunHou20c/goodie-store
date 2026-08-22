//! Product — photograph, the price, the spec table, and the desk's note.

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::catalog::{Catalog, Product};
use crate::ui::{Kicker, Photo, WithCatalog};

/// The `:id` segment of `/p/:id` — a product slug.
pub fn slug_param() -> Memo<String> {
    let params = use_params_map();
    Memo::new(move |_| params.read().get("id").unwrap_or_default())
}

#[component]
pub fn ProductScreen() -> impl IntoView {
    let catalog = Catalog::from_context();
    let slug = slug_param();

    view! {
        <WithCatalog>
            {move || match catalog.by_slug(&slug.get()) {
                Some(product) => view! { <ProductBody product /> }.into_any(),
                None => {
                    view! {
                        <div class="px-[18px] py-12">
                            <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">
                                "We do not stock that."
                            </div>
                            <p class="mt-2 mb-4 text-[13px] leading-[1.6] text-ink/62">
                                "It may have sold out and left the shelf."
                            </p>
                            <A href="/" {..} class="btn btn-primary no-underline">
                                "Back to Main Page"
                            </A>
                        </div>
                    }
                        .into_any()
                }
            }}
        </WithCatalog>
    }
}

#[component]
fn ProductBody(product: Product) -> impl IntoView {
    // Pull everything out first: the view macro wraps each block in a closure,
    // and `product` cannot be moved into more than one of them.
    let category = product.category_label();
    let num = product.num();
    let title = product.title.clone();
    let price = product.price_label();
    let availability = product.availability.clone();
    let description = product.description.clone();
    let thumbnail = product.thumbnail_url.clone();
    let specs = product.specs();
    let note = product.note.clone();

    view! {
        <div>
            <div class="h-[300px]">
                <Photo src=thumbnail label="Product photo" />
            </div>

            <div class="border-b-2 border-ink/40 px-[18px] pt-[18px] pb-4">
                <Kicker class="text-accent-700">{category} " · " {num}</Kicker>
                <h2 class="mt-2 text-[30px] leading-[1.05] tracking-[-0.025em]">{title}</h2>
                <div class="mt-2.5 flex items-baseline gap-3">
                    <div class="font-heading text-[22px] font-extrabold">{price}</div>
                    <div class="text-xs text-ink/55">{availability}</div>
                </div>
                <p class="mt-3.5 text-[13.5px] leading-[1.65] text-pretty">{description}</p>
            </div>

            <div class="border-b border-ink/18 px-[18px] py-4">
                <Kicker class="mb-2.5">"Specification"</Kicker>
                {specs
                    .into_iter()
                    .map(|(key, value)| {
                        view! {
                            <div class="grid grid-cols-[1fr_auto] gap-4 border-t border-ink/14 py-[9px] text-[12.5px]">
                                <div class="text-ink/58">{key}</div>
                                <div class="text-right font-semibold">{value}</div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>

            // The buying desk's note is editorial and not part of the seed, so
            // the panel only appears once someone has written one.
            {note
                .map(|note| {
                    view! {
                        <div class="border-b-2 border-ink/40 bg-surface p-[18px]">
                            <Kicker class="text-accent-700">"Why we stock it"</Kicker>
                            <p class="mt-2.5 text-sm leading-[1.6] text-pretty">{note}</p>
                            <div class="mt-3 text-[11.5px] uppercase tracking-[0.06em] text-ink/55">
                                "Written by the buying desk"
                            </div>
                        </div>
                    }
                })}
            <div class="h-3"></div>
        </div>
    }
}
