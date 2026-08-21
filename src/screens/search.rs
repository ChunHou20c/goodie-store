//! Search — filters first: chips for category, price and availability, and a
//! sort that cycles rather than opening a menu.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::catalog::CATS;
use crate::shop::Shop;
use crate::ui::{IconClose, IconSearch, IconSort, Photo};

#[component]
pub fn SearchScreen() -> impl IntoView {
    let shop = Shop::from_context();
    let results = move || shop.results();

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
                {CATS
                    .iter()
                    .map(|chip| {
                        let chip = *chip;
                        let on = move || shop.filters.with(|f| f.contains(&chip));
                        view! {
                            <button
                                class=move || {
                                    if on() {
                                        "flex-none cursor-pointer border-2 border-accent bg-accent px-3 py-[7px] text-xs font-semibold uppercase tracking-[0.06em] text-ground"
                                    } else {
                                        "flex-none cursor-pointer border-2 border-ink/30 px-3 py-[7px] text-xs font-semibold uppercase tracking-[0.06em] text-ink hover:bg-ink/7"
                                    }
                                }
                                aria-pressed=move || if on() { "true" } else { "false" }
                                on:click=move |_| shop.toggle_filter(chip)
                            >
                                {chip}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>

            <div class="flex items-center justify-between border-b-2 border-ink/40 px-[18px] pt-1.5 pb-3">
                <div class="text-xs font-semibold">
                    {move || {
                        let n = results().len();
                        format!("{n} {}", if n == 1 { "object" } else { "objects" })
                    }}
                </div>
                <button
                    class="flex cursor-pointer items-center gap-1.5 text-xs font-semibold uppercase tracking-[0.06em] text-accent-700 hover:text-accent"
                    on:click=move |_| shop.sort.update(|s| *s = s.next())
                >
                    <IconSort />
                    {move || shop.sort.get().label()}
                </button>
            </div>

            {move || {
                let found = results();
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
                                            href=format!("/p/{}", p.id)
                                            {..}
                                            class="flex flex-col gap-2.5 bg-ground p-3.5 text-left text-ink no-underline transition-colors hover:bg-accent-100"
                                        >
                                            <div class="h-[118px] border border-ink/14">
                                                <Photo label="Photo" />
                                            </div>
                                            <div>
                                                <div class="text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-700">
                                                    {p.cat}
                                                </div>
                                                <div class="mt-[5px] font-heading text-sm font-extrabold leading-tight">
                                                    {p.name}
                                                </div>
                                                <div class="mt-1.5 text-[13px] font-semibold">
                                                    {p.price_label()}
                                                </div>
                                                <div class="mt-0.5 text-[11px] text-ink/55">{p.stock}</div>
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
            <div class="h-5"></div>
        </div>
    }
}
