//! Admin console — pull more of the upstream catalogue into the database.
//!
//! The screen is hidden from non-admins, but that is presentation. The privilege
//! boundary is `require_admin()` inside [`crate::catalog::import_products`].

use leptos::prelude::*;
use leptos_router::components::A;

use crate::auth::Auth;
use crate::catalog::{Catalog, ImportProducts};
use crate::screens::login::FormError;
use crate::ui::{IconArrowRight, Kicker};

#[component]
pub fn AdminScreen() -> impl IntoView {
    let auth = Auth::from_context();

    view! {
        <Suspense fallback=|| {
            view! { <div class="px-[18px] py-12 text-[13px] text-ink/55">"…"</div> }
        }>
            {move || {
                if auth.is_admin() {
                    view! { <ImportConsole /> }.into_any()
                } else {
                    view! { <NotForYou signed_in=auth.is_signed_in() /> }.into_any()
                }
            }}
        </Suspense>
    }
}

#[component]
fn ImportConsole() -> impl IntoView {
    let catalog = Catalog::from_context();
    let import = ServerAction::<ImportProducts>::new();

    // The obvious click imports the next batch: start where the catalogue ends.
    let next_offset = move || catalog.len();

    // A finished import changes the catalogue, so re-read it.
    Effect::new(move |_| {
        if matches!(import.value().get(), Some(Ok(_))) {
            catalog.refetch();
        }
    });

    view! {
        <div>
            <div class="border-b-2 border-ink/40 px-[18px] pt-[18px] pb-4">
                <Kicker class="text-accent-700">"Admin"</Kicker>
                <h2 class="mt-2 text-[30px] leading-[1.05] tracking-[-0.025em]">
                    "Import products"
                </h2>
                <p class="mt-2.5 text-[13px] leading-[1.6] text-ink/62 text-pretty">
                    "Pulls a slice of the dummyjson catalogue into the database. Products already
                     here are refreshed rather than duplicated, and editorial notes are left alone."
                </p>
            </div>

            <div class="border-b border-ink/18 px-[18px] py-3.5">
                <div class="flex items-baseline justify-between text-[12.5px]">
                    <span class="text-ink/60">"In the database"</span>
                    <span class="font-heading font-extrabold">
                        {move || format!("{} objects", catalog.len())}
                    </span>
                </div>
            </div>

            <div class="px-[18px] py-[18px]">
                <ActionForm action=import>
                    <div class="grid grid-cols-2 gap-3">
                        // `limit` carries the server's clamp; `skip` walks the whole
                        // upstream catalogue, so it must not inherit that ceiling.
                        <NumberField label="How many" name="limit" value=30 min=1 max=Some(100) />
                        <NumberField
                            label="Starting at"
                            name="skip"
                            value=Signal::derive(next_offset)
                            min=0
                        />
                    </div>
                    <FormError message=move || {
                        match import.value().get() {
                            Some(Err(e)) => Some(e.to_string()),
                            _ => None,
                        }
                    } />
                    <button
                        type="submit"
                        class="btn btn-primary mt-3.5 w-full justify-between px-4 py-3.5"
                        disabled=move || import.pending().get()
                    >
                        {move || {
                            if import.pending().get() {
                                "Importing…"
                            } else {
                                "Import from dummyjson"
                            }
                        }}
                        <IconArrowRight />
                    </button>
                </ActionForm>

                {move || {
                    match import.value().get() {
                        Some(Ok(report)) => {
                            view! {
                                <div class="mt-4 border-t-2 border-ink/40 pt-3.5">
                                    <Kicker>"Last run"</Kicker>
                                    <div class="mt-2 font-heading text-lg font-extrabold leading-tight">
                                        {report.summary()}
                                    </div>
                                    <div class="mt-1.5 text-[12px] text-ink/55">
                                        {format!("{} rows fetched upstream", report.fetched)}
                                    </div>
                                </div>
                            }
                                .into_any()
                        }
                        _ => ().into_any(),
                    }
                }}
            </div>
            <div class="h-5"></div>
        </div>
    }
}

#[component]
fn NotForYou(signed_in: bool) -> impl IntoView {
    view! {
        <div class="px-[18px] py-12">
            <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">
                "This is the buying desk."
            </div>
            <p class="mt-2 mb-4 text-[13px] leading-[1.6] text-ink/62 text-pretty">
                {if signed_in {
                    "Your account does not have admin access."
                } else {
                    "Sign in with an admin account to reach the console."
                }}
            </p>
            <A href="/login" {..} class="btn btn-primary no-underline">
                {if signed_in { "Back to your account" } else { "Sign in" }}
            </A>
        </div>
    }
}

#[component]
fn NumberField(
    label: &'static str,
    name: &'static str,
    #[prop(into)] value: Signal<usize>,
    #[prop(default = 0)] min: u32,
    /// Leave unset for an unbounded field. A `max` that the field's own default
    /// value exceeds makes the browser refuse to submit the form at all.
    #[prop(default = None)]
    max: Option<u32>,
) -> impl IntoView {
    view! {
        <label class="block">
            <span class="mb-1.5 block text-[11px] font-semibold uppercase tracking-[0.14em] text-ink/60">
                {label}
            </span>
            <input
                type="number"
                name=name
                min=min
                max=max
                prop:value=move || value.get().to_string()
                class="w-full border-2 border-ink bg-transparent px-3 py-[11px] text-[14.5px] font-semibold text-ink caret-accent outline-none focus-visible:border-accent"
            />
        </label>
    }
}
