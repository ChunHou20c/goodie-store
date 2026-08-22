//! Search — filters first: chips for category, price and availability, and a
//! sort that cycles rather than opening a menu. All of it runs in the browser
//! over the catalogue the server sent.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::catalog::{category_label, chips, Catalog, IN_STOCK, UNDER_400};
use crate::shop::Shop;
use crate::ui::{IconClose, IconSearch, IconSort, Photo, WithCatalog};

/// Category chips print as their prettified label; the two others are already
/// written the way they should read.
fn chip_label(chip: &str) -> String {
    if chip == UNDER_400 || chip == IN_STOCK {
        chip.to_string()
    } else {
        category_label(chip)
    }
}

#[component]
pub fn SearchScreen() -> impl IntoView {
    let shop = Shop::from_context();
    let catalog = Catalog::from_context();

    view! {
        <div>
            <div class="border-b border-ink/18 px-[18px] pt-4 pb-3.5">
                <div class="flex items-center gap-2.5 border-2 border-ink px-3 py-[11px]">
                    <IconSearch size=18 />
                    <input
                        class="min-w-0 flex-1 bg-transparent text-[14.5px] font-semibold text-ink caret-accent outline-none placeholder:font-normal placeholder:text-ink/45"
                        placeholder="Search the index"
                        aria-label="Search the index"
                        prop:value=move || shop.query.get()
                        on:input=move |ev| shop.query.set(event_target_value(&ev))
                    />
                    <Show when=move || !shop.query.with(String::is_empty)>
                        <button
                            class="flex cursor-pointer text-ink/55 hover:text-accent"
                            aria-label="Clear search"
                            on:click=move |_| shop.query.set(String::new())
                        >
                            <IconClose />
                        </button>
                    </Show>
                </div>
            </div>

            <div class="flex gap-2 overflow-x-auto px-[18px] py-3">
                <Suspense fallback=|| ()>
                    {move || {
                        catalog
                            .with(chips)
                            .into_iter()
                            .map(|chip| {
                                let label = chip_label(&chip);
                                let (on_chip, click_chip) = (chip.clone(), chip.clone());
                                let on = Memo::new(move |_| shop.is_filtered_by(&on_chip));
                                // A Memo is Copy, so both attribute closures can read it.
                                view! {
                                    <button
                                        class=move || {
                                            if on.get() {
                                                "flex-none cursor-pointer border-2 border-accent bg-accent px-3 py-[7px] text-xs font-semibold whitespace-nowrap uppercase tracking-[0.06em] text-ground"
                                            } else {
                                                "flex-none cursor-pointer border-2 border-ink/30 px-3 py-[7px] text-xs font-semibold whitespace-nowrap uppercase tracking-[0.06em] text-ink hover:bg-ink/7"
                                            }
                                        }
                                        aria-pressed=move || if on.get() { "true" } else { "false" }
                                        on:click=move |_| shop.toggle_filter(&click_chip)
                                    >
                                        {label}
                                    </button>
                                }
                            })
                            .collect_view()
                    }}
                </Suspense>
            </div>

            <div class="flex items-center justify-between border-b-2 border-ink/40 px-[18px] pt-1.5 pb-3">
                <div class="text-xs font-semibold">
                    <Suspense fallback=|| ()>
                        {move || {
                            let n = catalog.with(|products| shop.results(products).len());
                            format!("{n} {}", if n == 1 { "object" } else { "objects" })
                        }}
                    </Suspense>
                </div>
                <button
                    class="flex cursor-pointer items-center gap-1.5 text-xs font-semibold uppercase tracking-[0.06em] text-accent-700 hover:text-accent"
                    on:click=move |_| shop.sort.update(|s| *s = s.next())
                >
                    <IconSort />
                    {move || shop.sort.get().label()}
                </button>
            </div>

            <WithCatalog>
                {move || {
                    let found = catalog
                        .with(|products| {
                            shop.results(products).into_iter().cloned().collect::<Vec<_>>()
                        });
                    if found.is_empty() {
                        view! {
                            <div class="border-b-2 border-ink/40 px-[18px] py-10">
                                <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">
                                    "Nothing under that name yet."
                                </div>
                                <p class="mt-2 mb-4 text-[13px] leading-[1.6] text-ink/62 text-pretty">
                                    "Try clearing a filter — or tell us what you're after and we'll write back when it lands."
                                </p>
                                <button
                                    class="btn btn-secondary"
                                    on:click=move |_| shop.reset_filters()
                                >
                                    "Clear filters"
                                </button>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="grid grid-cols-2 gap-px bg-ink/18">
                                {found
                                    .into_iter()
                                    .map(|p| {
                                        view! {
                                            <A
                                                href=format!("/p/{}", p.slug)
                                                {..}
                                                class="flex flex-col gap-2.5 bg-ground p-3.5 text-left text-ink no-underline transition-colors hover:bg-accent-100"
                                            >
                                                <div class="h-[118px] border border-ink/14">
                                                    <Photo src=p.thumbnail_url.clone() label="Photo" />
                                                </div>
                                                <div>
                                                    <div class="text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-700">
                                                        {p.category_label()}
                                                    </div>
                                                    <div class="mt-[5px] font-heading text-sm font-extrabold leading-tight">
                                                        {p.title.clone()}
                                                    </div>
                                                    <div class="mt-1.5 text-[13px] font-semibold">
                                                        {p.price_label()}
                                                    </div>
                                                    <div class="mt-0.5 text-[11px] text-ink/55">
                                                        {p.availability.clone()}
                                                    </div>
                                                </div>
                                            </A>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                            .into_any()
                    }
                }}
            </WithCatalog>
            <div class="h-5"></div>
        </div>
    }
}
