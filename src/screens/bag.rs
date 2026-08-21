//! Bag — reversible: steppers, remove, and a total that carries the delivery
//! promise so nothing new appears at checkout.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::catalog::money;
use crate::shop::Shop;
use crate::ui::Photo;

#[component]
pub fn BagScreen() -> impl IntoView {
    let shop = Shop::from_context();

    view! {
        <div>
            <div class="border-b-2 border-ink/40 px-[18px] pt-[18px] pb-3.5">
                <h2 class="text-[28px] tracking-[-0.02em]">"Your bag"</h2>
                <div class="mt-1.5 text-[12.5px] text-ink/60">
                    {move || {
                        match shop.count() {
                            0 => "Free returns for 30 days".to_string(),
                            1 => "1 item · arrives Tue 25 Aug".to_string(),
                            n => format!("{n} items · arrive Tue 25 Aug"),
                        }
                    }}
                </div>
            </div>

            {move || {
                shop.lines()
                    .into_iter()
                    .map(|(p, qty)| {
                        view! {
                            <div class="grid grid-cols-[74px_1fr] gap-3.5 border-b border-ink/18 px-[18px] py-4">
                                <div class="h-[74px] border border-ink/14">
                                    <Photo />
                                </div>
                                <div>
                                    <div class="flex justify-between gap-3">
                                        <div class="font-heading text-[15px] font-extrabold leading-tight">
                                            {p.name}
                                        </div>
                                        <div class="font-heading text-sm font-extrabold whitespace-nowrap">
                                            {money(p.price * qty)}
                                        </div>
                                    </div>
                                    <div class="mt-[3px] text-[11.5px] text-ink/55">
                                        {p.cat} " · " {p.price_label()} " each"
                                    </div>
                                    <div class="mt-3 flex items-center justify-between">
                                        <div class="flex items-center border-2 border-ink">
                                            <button
                                                class="h-8 w-[34px] cursor-pointer font-heading text-[17px] font-extrabold hover:bg-ink/7"
                                                aria-label=format!("One fewer {}", p.name)
                                                on:click=move |_| shop.bump(p.id, -1)
                                            >
                                                "−"
                                            </button>
                                            <div class="w-[30px] text-center text-[13.5px] font-extrabold">
                                                {qty}
                                            </div>
                                            <button
                                                class="h-8 w-[34px] cursor-pointer font-heading text-[17px] font-extrabold hover:bg-ink/7"
                                                aria-label=format!("One more {}", p.name)
                                                on:click=move |_| shop.bump(p.id, 1)
                                            >
                                                "+"
                                            </button>
                                        </div>
                                        <button
                                            class="cursor-pointer text-[11.5px] font-semibold uppercase tracking-[0.08em] text-accent-700 hover:text-accent"
                                            on:click=move |_| shop.remove(p.id)
                                        >
                                            "Remove"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                    .collect_view()
            }}

            <Show
                when=move || { shop.count() > 0 }
                fallback=move || {
                    view! {
                        <div class="px-[18px] py-11">
                            <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">
                                "Nothing in here yet."
                            </div>
                            <p class="mt-2 mb-[18px] text-[13px] leading-[1.6] text-ink/62 text-pretty">
                                "Start with the index — six things we'd happily own twice."
                            </p>
                            <A href="/" {..} class="btn btn-primary no-underline">
                                "Back to Issue 14"
                            </A>
                        </div>
                    }
                }
            >
                <div class="p-[18px]">
                    <div class="flex justify-between py-[7px] text-[13px]">
                        <span class="text-ink/60">"Subtotal"</span>
                        <span class="font-semibold">{move || money(shop.subtotal())}</span>
                    </div>
                    <div class="flex justify-between border-t border-ink/14 py-[7px] text-[13px]">
                        <span class="text-ink/60">"Delivery — 2 days, signed for"</span>
                        <span class="font-semibold">"Free"</span>
                    </div>
                    <div class="mt-2 flex items-baseline justify-between border-t-2 border-ink/40 pt-3">
                        <span class="text-xs font-semibold uppercase tracking-[0.14em]">
                            "Total"
                        </span>
                        <span class="font-heading text-2xl font-extrabold">
                            {move || money(shop.subtotal())}
                        </span>
                    </div>
                </div>
            </Show>
            <div class="h-5"></div>
        </div>
    }
}
