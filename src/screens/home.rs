//! Home — the issue: a cover story, then the numbered index.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::catalog::Catalog;
use crate::ui::{IconArrowRight, Kicker, Photo, WithCatalog};

#[component]
pub fn HomeScreen() -> impl IntoView {
    let catalog = Catalog::from_context();

    view! {
        <div>
            <div class="flex items-baseline justify-between px-[18px] pt-[18px]">
                <Kicker class="text-accent-700">"Issue 14 · Aug"</Kicker>
                <div class="text-[11px] font-semibold uppercase tracking-[0.1em] text-ink/50">
                    "Free returns, always"
                </div>
            </div>

            <div class="px-[18px] pt-2.5">
                <h2 class="mb-2.5 text-[34px] leading-[1.03] tracking-[-0.025em]">
                    "The Quiet Machine"
                </h2>
                <p class="mb-4 text-[13.5px] leading-[1.6] text-ink/68 text-pretty">
                    "The objects we keep coming back to — chosen for the way they sound, feel and age, not for the spec sheet."
                </p>
            </div>

            <div class="border-b-2 border-ink/40 px-[18px] pb-[18px]">
                <A
                    href="/search"
                    {..}
                    class="btn btn-primary w-full justify-between px-4 py-3.5 tracking-[0.04em]"
                >
                    "Read the whole issue"
                    <IconArrowRight />
                </A>
            </div>

            <div class="flex items-baseline justify-between px-[18px] pt-4 pb-2">
                <Kicker>"The index"</Kicker>
                <div class="text-[11px] text-ink/50">
                    // Everything the boundary needs to print goes inside it: a
                    // Suspense sibling to a static text node shifts the node walk
                    // during hydration and the text lands on a marker comment.
                    <Suspense fallback=|| ()>
                        {move || format!("{} objects", catalog.len())}
                    </Suspense>
                </div>
            </div>

            <WithCatalog>
                {move || {
                    catalog
                        .take(4)
                        .into_iter()
                        .map(|p| {
                            view! {
                                <A
                                    href=format!("/p/{}", p.slug)
                                    {..}
                                    class="grid grid-cols-[34px_1fr_92px] items-center gap-3.5 border-t border-ink/18 px-[18px] py-3.5 text-left text-ink no-underline transition-colors hover:bg-accent-100"
                                >
                                    <div class="text-xs font-extrabold tracking-[0.06em] text-accent-700">
                                        {p.num()}
                                    </div>
                                    <div>
                                        <div class="font-heading text-base font-extrabold leading-tight tracking-[-0.01em]">
                                            {p.title.clone()}
                                        </div>
                                        <div class="mt-[3px] line-clamp-2 text-xs leading-[1.5] text-ink/58">
                                            {p.line()}
                                        </div>
                                        <div class="mt-1.5 text-[12.5px] font-semibold">
                                            {p.price_label()}
                                        </div>
                                    </div>
                                    <div class="h-[76px]">
                                        <Photo src=p.thumbnail_url.clone() label="Photo" />
                                    </div>
                                </A>
                            }
                        })
                        .collect_view()
                }}
            </WithCatalog>

            <div class="border-t-2 border-ink/40 bg-accent p-[18px] text-ground">
                <Kicker class="opacity-85">"Dispatch"</Kicker>
                <div class="my-2 font-heading text-2xl font-extrabold leading-[1.1] tracking-[-0.02em]">
                    "“Buy it once. We'll help you keep it running.”"
                </div>
                <div class="text-[12.5px] leading-[1.55] opacity-90">
                    "Five-year parts guarantee on everything in the index — repairs booked from your orders page."
                </div>
            </div>
            <div class="h-5"></div>
        </div>
    }
}
