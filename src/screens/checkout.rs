//! Checkout — the page that holds stock and takes the payment.
//!
//! Three states, in order of precedence: a receipt when `?order=` names one of
//! your orders, the pending reservation when you have one, and otherwise
//! nothing to pay for. The receipt is keyed off the query string rather than
//! off the action's value so that reloading the page still shows it.

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::checkout::{get_order, Checkout, Line, OrderView};
use crate::screens::login::FormError;
use crate::ui::Kicker;

#[component]
pub fn CheckoutScreen() -> impl IntoView {
    let checkout = Checkout::from_context();

    let query = use_query_map();
    let order_id = move || {
        query
            .read()
            .get("order")
            .and_then(|v| v.parse::<i64>().ok())
    };
    let order = Resource::new_blocking(order_id, |id| async move {
        match id {
            Some(id) => get_order(id).await,
            None => Ok(None),
        }
    });

    // A settled payment turns into a receipt URL, so a reload still shows it.
    let navigate = use_navigate();
    Effect::new(move |ran_before: Option<()>| {
        // Read first, guard second: a short-circuiting guard would stop this
        // effect ever subscribing to the action.
        let paid = checkout.pay.value().get();
        if let (true, Some(Ok(id))) = (ran_before.is_some(), paid) {
            navigate(&format!("/checkout?order={id}"), Default::default());
        }
    });

    view! {
        <Suspense fallback=|| {
            view! { <div class="px-[18px] py-12 text-[13px] text-ink/55">"…"</div> }
        }>
            {move || {
                match order.get().and_then(Result::ok).flatten() {
                    Some(order) => view! { <Receipt order /> }.into_any(),
                    None => {
                        match checkout.pending_reservation() {
                            Some(reservation) => {
                                let (expires, total) = (
                                    reservation.expires_label(),
                                    reservation.total_label(),
                                );
                                view! {
                                    <div>
                                        <div class="border-b-2 border-ink/40 px-[18px] pt-[18px] pb-4">
                                            <Kicker class="text-accent-700">"Checkout"</Kicker>
                                            <h2 class="mt-2 text-[30px] leading-[1.05] tracking-[-0.025em]">
                                                "Payment pending"
                                            </h2>
                                            <p class="mt-2.5 text-[12.5px] text-ink/60">
                                                "These are held for you — " {expires} "."
                                            </p>
                                        </div>
                                        <Lines lines=reservation.lines.clone() />
                                        <div class="px-[18px]">
                                            <FormError message=move || checkout.last_error() />
                                        </div>
                                        <Total label=total caption="Due now" />
                                    </div>
                                }
                                    .into_any()
                            }
                            None => {
                                view! {
                                    <div class="px-[18px] py-12">
                                        <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">
                                            "Nothing to pay for."
                                        </div>
                                        <p class="mt-2 mb-[18px] text-[13px] leading-[1.6] text-ink/62 text-pretty">
                                            "Fill the bag and check out — we hold what you pick for 15 minutes."
                                        </p>
                                        <A href="/" {..} class="btn btn-primary no-underline">
                                            "Back to Main Page"
                                        </A>
                                    </div>
                                }
                                    .into_any()
                            }
                        }
                    }
                }
            }}
        </Suspense>
    }
}

#[component]
fn Receipt(order: OrderView) -> impl IntoView {
    let (id, total) = (order.id, order.total_label());

    view! {
        <div>
            <div class="border-b-2 border-ink/40 px-[18px] pt-[18px] pb-4">
                <Kicker class="text-accent-700">"Paid"</Kicker>
                <h2 class="mt-2 text-[30px] leading-[1.05] tracking-[-0.025em]">"Order #" {id}</h2>
                <p class="mt-2.5 text-[12.5px] text-ink/60">
                    "Thank you — these are off the shelf and on the way."
                </p>
            </div>
            <Lines lines=order.lines.clone() />
            <Total label=total caption="Paid" />
            <div class="px-[18px] pb-[18px]">
                <A href="/" {..} class="btn btn-secondary w-full justify-center no-underline">
                    "Keep shopping"
                </A>
            </div>
        </div>
    }
}

#[component]
fn Lines(lines: Vec<Line>) -> impl IntoView {
    view! {
        {lines
            .into_iter()
            .map(|line| {
                view! {
                    <div class="flex items-baseline justify-between gap-3 border-b border-ink/18 px-[18px] py-3.5">
                        <div>
                            <div class="font-heading text-[15px] font-extrabold leading-tight">
                                {line.title.clone()}
                            </div>
                            <div class="mt-[3px] text-[11.5px] text-ink/55">
                                {line.quantity} " × "
                                {crate::catalog::money(line.unit_price_cents)}
                            </div>
                        </div>
                        <div class="font-heading text-sm font-extrabold whitespace-nowrap">
                            {line.subtotal_label()}
                        </div>
                    </div>
                }
            })
            .collect_view()}
    }
}

#[component]
fn Total(label: String, caption: &'static str) -> impl IntoView {
    view! {
        <div class="p-[18px]">
            <div class="flex justify-between border-t border-ink/14 py-[7px] text-[13px]">
                <span class="text-ink/60">"Delivery — 2 days, signed for"</span>
                <span class="font-semibold">"Free"</span>
            </div>
            <div class="mt-2 flex items-baseline justify-between border-t-2 border-ink/40 pt-3">
                <span class="text-xs font-semibold uppercase tracking-[0.14em]">{caption}</span>
                <span class="font-heading text-2xl font-extrabold">{label}</span>
            </div>
        </div>
    }
}
