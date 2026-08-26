//! Orders — what you bought, and what you let lapse.
//!
//! Two sections from one round trip: placed orders, and checkouts that ran out
//! of time before they were paid for. The lapsed ones are here because expiry
//! does not put the bag back, so this is the only record of what was in one.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::auth::Auth;
use crate::checkout::{order_history, ExpiredReservation, History, Line, OrderSummary};
use crate::ui::Kicker;

#[component]
pub fn OrdersScreen() -> impl IntoView {
    let auth = Auth::from_context();
    // Its own resource rather than a context struct: nothing outside this
    // screen reads the history, and no action here writes.
    let history = Resource::new_blocking(|| (), |_| order_history());

    view! {
        <div>
            <div class="border-b-2 border-ink/40 px-[18px] pt-[18px] pb-4">
                <Kicker class="text-accent-700">"Account"</Kicker>
                <h2 class="mt-2 text-[30px] leading-[1.05] tracking-[-0.025em]">"Your orders"</h2>
            </div>

            <Suspense fallback=|| {
                view! { <div class="px-[18px] py-12 text-[13px] text-ink/55">"…"</div> }
            }>
                {move || {
                    if !auth.is_signed_in() {
                        return view! { <SignInFirst /> }.into_any();
                    }
                    match history.get() {
                        Some(Ok(history)) => view! { <Sections history /> }.into_any(),
                        Some(Err(e)) => {
                            view! {
                                <div class="px-[18px] py-12">
                                    <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">
                                        "Your orders are unavailable."
                                    </div>
                                    <p class="mt-2 font-mono text-[11px] break-all text-ink/45">
                                        {e.to_string()}
                                    </p>
                                </div>
                            }
                                .into_any()
                        }
                        None => ().into_any(),
                    }
                }}
            </Suspense>
            <div class="h-5"></div>
        </div>
    }
}

#[component]
fn Sections(history: History) -> impl IntoView {
    if history.is_empty() {
        return view! {
            <div class="px-[18px] py-11">
                <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">
                    "Nothing bought yet."
                </div>
                <p class="mt-2 mb-[18px] text-[13px] leading-[1.6] text-ink/62 text-pretty">
                    "When you check out, the receipt lands here."
                </p>
                <A href="/" {..} class="btn btn-primary no-underline">
                    "Back to Main Page"
                </A>
            </div>
        }
        .into_any();
    }

    view! {
        <div>
            <Show when={
                let any = !history.orders.is_empty();
                move || any
            }>
                <div class="border-b border-ink/18 px-[18px] pt-[18px] pb-2">
                    <Kicker>"Placed"</Kicker>
                </div>
            </Show>
            {history
                .orders
                .iter()
                .cloned()
                .map(|order| view! { <OrderCard order /> })
                .collect_view()}

            <Show when={
                let any = !history.expired.is_empty();
                move || any
            }>
                <div class="border-b border-ink/18 px-[18px] pt-[18px] pb-2">
                    <Kicker class="text-ink/50">"Lapsed"</Kicker>
                    <p class="mt-1.5 text-[11.5px] leading-[1.5] text-ink/55">
                        "These checkouts ran out of time, so the stock went back on the shelf."
                    </p>
                </div>
            </Show>
            {history
                .expired
                .iter()
                .cloned()
                .map(|reservation| view! { <LapsedCard reservation /> })
                .collect_view()}
        </div>
    }
    .into_any()
}

#[component]
fn OrderCard(order: OrderSummary) -> impl IntoView {
    let (id, placed_on, total) = (order.id, order.placed_on.clone(), order.total_label());

    view! {
        <div class="border-b border-ink/18 px-[18px] py-4">
            <div class="flex items-baseline justify-between gap-3">
                <div class="font-heading text-[15px] font-extrabold">"Order #" {id}</div>
                <div class="font-heading text-sm font-extrabold whitespace-nowrap">{total}</div>
            </div>
            <div class="mt-[3px] text-[11.5px] text-ink/55">{placed_on}</div>
            <LineList lines=order.lines.clone() />
            <A
                href=format!("/checkout?order={id}")
                {..}
                class="mt-3 inline-block text-[11.5px] font-semibold uppercase tracking-[0.08em] text-accent-700 no-underline hover:text-accent"
            >
                "View receipt"
            </A>
        </div>
    }
}

#[component]
fn LapsedCard(reservation: ExpiredReservation) -> impl IntoView {
    let (expired_on, total) = (reservation.expired_on.clone(), reservation.total_label());

    view! {
        <div class="border-b border-ink/18 px-[18px] py-4 opacity-70">
            <div class="flex items-baseline justify-between gap-3">
                <div class="font-heading text-[15px] font-extrabold text-ink/70">"Expired"</div>
                <div class="font-heading text-sm font-extrabold whitespace-nowrap text-ink/70">
                    {total}
                </div>
            </div>
            <div class="mt-[3px] text-[11.5px] text-ink/55">{expired_on}</div>
            <LineList lines=reservation.lines.clone() />
        </div>
    }
}

#[component]
fn LineList(lines: Vec<Line>) -> impl IntoView {
    view! {
        <div class="mt-2.5 flex flex-col gap-1">
            {lines
                .into_iter()
                .map(|line| {
                    view! {
                        <div class="flex justify-between gap-3 text-[12.5px]">
                            <span class="text-ink/70">
                                {line.quantity} " × " {line.title.clone()}
                            </span>
                            <span class="whitespace-nowrap text-ink/55">
                                {line.subtotal_label()}
                            </span>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn SignInFirst() -> impl IntoView {
    view! {
        <div class="px-[18px] py-11">
            <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">
                "Your orders need an account."
            </div>
            <p class="mt-2 mb-[18px] text-[13px] leading-[1.6] text-ink/62 text-pretty">
                "Sign in and everything you have bought is listed here."
            </p>
            <A href="/login?next=/orders" {..} class="btn btn-primary no-underline">
                "Sign in"
            </A>
        </div>
    }
}
