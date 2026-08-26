//! Bag — reversible: steppers, remove, and a total that carries the delivery
//! promise so nothing new appears at checkout.
//!
//! The rows come from the server ([`crate::cart::Cart`]) and are priced against
//! the catalogue, so every tap is a round-trip and what you see is what the
//! database holds. A bag belongs to an account; signed out there is nothing to
//! show but the way in.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::auth::Auth;
use crate::cart::Cart;
use crate::catalog::{money, Catalog};
use crate::checkout::Checkout;
use crate::screens::login::FormError;
use crate::ui::{Photo, WithCatalog};

#[component]
pub fn BagScreen() -> impl IntoView {
    let cart = Cart::from_context();
    let auth = Auth::from_context();
    let checkout = Checkout::from_context();
    let catalog = Catalog::from_context();
    let subtotal = move || catalog.with(|products| money(cart.subtotal(products)));

    view! {
        <div>
            <div class="flex items-start justify-between gap-3 border-b-2 border-ink/40 px-[18px] pt-[18px] pb-3.5">
                <div>
                    <h2 class="text-[28px] tracking-[-0.02em]">"Your bag"</h2>
                    <div class="mt-1.5 text-[12.5px] text-ink/60">
                        // `count` reads a resource, so it needs a boundary.
                        <Suspense fallback=|| {
                            "Free returns for 30 days"
                        }>
                            {move || {
                                match cart.count() {
                                    0 => "Free returns for 30 days".to_string(),
                                    1 => "1 item · arrives Tue 25 Aug".to_string(),
                                    n => format!("{n} items · arrive Tue 25 Aug"),
                                }
                            }}
                        </Suspense>
                    </div>
                </div>
                // Always offered, signed in or not: /orders has its own
                // signed-out state that invites you to sign in.
                <A
                    href="/orders"
                    {..}
                    class="btn btn-secondary mt-1 flex-none px-3 py-2 text-[11px] tracking-[0.12em] no-underline"
                >
                    "ORDERS"
                </A>
            </div>

            // The pending checkout is a resource read, so it needs a boundary:
            // without one a fresh server render sees it unresolved and the card
            // silently never appears.
            <Suspense fallback=|| ()>
                <Show when=move || checkout.has_pending()>
                    <A
                        href="/checkout"
                        {..}
                        class="flex items-center justify-between gap-3 border-b-2 border-ink/40 bg-accent-100 px-[18px] py-3.5 no-underline"
                    >
                        <span>
                            <span class="block font-heading text-[14px] font-extrabold text-accent-800">
                                "Payment pending"
                            </span>
                            <span class="mt-[3px] block text-[11.5px] text-ink/62">
                                "Your last checkout is holding stock — finish it."
                            </span>
                        </span>
                        <span class="text-[11.5px] font-extrabold uppercase tracking-[0.1em] text-accent-700">
                            "Continue"
                        </span>
                    </A>
                </Show>
            </Suspense>

            <WithCatalog>
                {move || {
                    catalog
                        .with(|products| cart.rows(products))
                        .into_iter()
                        .map(|(p, qty)| {
                            let id = p.id;
                            let (dec_label, inc_label) = (
                                format!("One fewer {}", p.title),
                                format!("One more {}", p.title),
                            );
                            view! {
                                <div class="grid grid-cols-[74px_1fr] gap-3.5 border-b border-ink/18 px-[18px] py-4">
                                    <div class="h-[74px] border border-ink/14">
                                        <Photo src=p.thumbnail_url.clone() />
                                    </div>
                                    <div>
                                        <div class="flex justify-between gap-3">
                                            <div class="font-heading text-[15px] font-extrabold leading-tight">
                                                {p.title.clone()}
                                            </div>
                                            <div class="font-heading text-sm font-extrabold whitespace-nowrap">
                                                {money(p.price_cents * qty)}
                                            </div>
                                        </div>
                                        <div class="mt-[3px] text-[11.5px] text-ink/55">
                                            {p.category_label()} " · " {p.price_label()} " each"
                                        </div>
                                        <div class="mt-3 flex items-center justify-between">
                                            <div class="flex items-center border-2 border-ink">
                                                // Every stepper is a write; disabling
                                                // while one is in flight keeps a burst
                                                // of taps from racing the refetch.
                                                <button
                                                    class="h-8 w-[34px] cursor-pointer font-heading text-[17px] font-extrabold hover:bg-ink/7 disabled:cursor-not-allowed disabled:opacity-45"
                                                    aria-label=dec_label
                                                    disabled=move || cart.pending()
                                                    on:click=move |_| cart.set(id, qty - 1)
                                                >
                                                    "−"
                                                </button>
                                                <div class="w-[30px] text-center text-[13.5px] font-extrabold">
                                                    {qty}
                                                </div>
                                                <button
                                                    class="h-8 w-[34px] cursor-pointer font-heading text-[17px] font-extrabold hover:bg-ink/7 disabled:cursor-not-allowed disabled:opacity-45"
                                                    aria-label=inc_label
                                                    disabled=move || cart.pending()
                                                    on:click=move |_| cart.set(id, qty + 1)
                                                >
                                                    "+"
                                                </button>
                                            </div>
                                            <button
                                                class="cursor-pointer text-[11.5px] font-semibold uppercase tracking-[0.08em] text-accent-700 hover:text-accent disabled:cursor-not-allowed disabled:opacity-45"
                                                disabled=move || cart.pending()
                                                on:click=move |_| cart.remove_item(id)
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
            </WithCatalog>

            <Suspense fallback=|| ()>
                <div class="px-[18px]">
                    <FormError message=move || cart.last_error() />
                </div>
                <Show
                    when=move || { cart.count() > 0 }
                    fallback=move || {
                        view! { <EmptyBag signed_in=auth.is_signed_in() /> }
                    }
                >
                    <div class="p-[18px]">
                        <div class="flex justify-between py-[7px] text-[13px]">
                            <span class="text-ink/60">"Subtotal"</span>
                            <span class="font-semibold">{subtotal}</span>
                        </div>
                        <div class="flex justify-between border-t border-ink/14 py-[7px] text-[13px]">
                            <span class="text-ink/60">"Delivery — 2 days, signed for"</span>
                            <span class="font-semibold">"Free"</span>
                        </div>
                        <div class="mt-2 flex items-baseline justify-between border-t-2 border-ink/40 pt-3">
                            <span class="text-xs font-semibold uppercase tracking-[0.14em]">
                                "Total"
                            </span>
                            <span class="font-heading text-2xl font-extrabold">{subtotal}</span>
                        </div>
                    </div>
                </Show>
            </Suspense>
            <div class="h-5"></div>
        </div>
    }
}

/// Two different nothings: a signed-in shopper has an empty bag, a visitor has
/// no bag at all.
#[component]
fn EmptyBag(signed_in: bool) -> impl IntoView {
    let (heading, body, href, label) = if signed_in {
        (
            "Nothing in here yet.",
            "Start with the shelf — things we'd happily own twice.",
            "/",
            "Back to Main Page",
        )
    } else {
        (
            "Your bag needs an account.",
            "Sign in and it follows you — this browser, your phone, next week.",
            "/login?next=/bag",
            "Sign in",
        )
    };

    view! {
        <div class="px-[18px] py-11">
            <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">{heading}</div>
            <p class="mt-2 mb-[18px] text-[13px] leading-[1.6] text-ink/62 text-pretty">{body}</p>
            <A href={href} {..} class="btn btn-primary no-underline">
                {label}
            </A>
        </div>
    }
}
