//! Shared pieces: the Lucide icon set the design uses, and the photo slot.

use leptos::prelude::*;

use crate::catalog::{Catalog, Status};

/// A photograph in the Modernist system: black and white, filling its
/// container, captioned bottom-left while there is nothing in it yet.
#[component]
pub fn Photo(
    /// The catalogue's image for this product, when it has one.
    #[prop(default = None)]
    src: Option<String>,
    #[prop(optional)] label: &'static str,
    /// Extra classes for the frame — a border, usually.
    #[prop(optional)]
    class: &'static str,
) -> impl IntoView {
    match src.filter(|url| !url.is_empty()) {
        Some(url) => view! {
            <div class=format!("photo h-full w-full bg-neutral-200 {class}")>
                <img src=url alt="" loading="lazy" class="h-full w-full object-contain" />
            </div>
        }
        .into_any(),
        None => view! {
            <div class=format!("photo flex h-full w-full items-end bg-neutral-200 p-2 {class}")>
                <Show when=move || !label.is_empty()>
                    <span class="text-[10px] font-semibold uppercase tracking-[0.12em] text-neutral-600">
                        {label}
                    </span>
                </Show>
            </div>
        }
        .into_any(),
    }
}

/// The small uppercase label that opens most sections.
#[component]
pub fn Kicker(#[prop(optional)] class: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class=format!(
            "text-[11px] font-semibold uppercase tracking-[0.18em] {class}",
        )>{children()}</div>
    }
}

#[component]
pub fn IconBack(#[prop(default = 16)] size: u32) -> impl IntoView {
    view! {
        <svg
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.4"
            aria-hidden="true"
        >
            <path d="M19 12H5" />
            <path d="m12 19-7-7 7-7" />
        </svg>
    }
}

#[component]
pub fn IconSearch(#[prop(default = 20)] size: u32) -> impl IntoView {
    view! {
        <svg
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            aria-hidden="true"
        >
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-3.5-3.5" />
        </svg>
    }
}

#[component]
pub fn IconBag(#[prop(default = 20)] size: u32) -> impl IntoView {
    view! {
        <svg
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            aria-hidden="true"
        >
            <path d="M6 2 4 6v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V6l-2-4z" />
            <path d="M4 6h16" />
            <path d="M16 10a4 4 0 0 1-8 0" />
        </svg>
    }
}

#[component]
pub fn IconArrowRight(#[prop(default = 18)] size: u32) -> impl IntoView {
    view! {
        <svg
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.4"
            aria-hidden="true"
        >
            <path d="M5 12h14" />
            <path d="m12 5 7 7-7 7" />
        </svg>
    }
}

#[component]
pub fn IconClose(#[prop(default = 17)] size: u32) -> impl IntoView {
    view! {
        <svg
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.4"
            aria-hidden="true"
        >
            <path d="M18 6 6 18" />
            <path d="m6 6 12 12" />
        </svg>
    }
}

#[component]
pub fn IconSort(#[prop(default = 15)] size: u32) -> impl IntoView {
    view! {
        <svg
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.4"
            aria-hidden="true"
        >
            <path d="M3 6h18" />
            <path d="M6 12h12" />
            <path d="M10 18h4" />
        </svg>
    }
}

#[component]
pub fn IconUser(#[prop(default = 20)] size: u32) -> impl IntoView {
    view! {
        <svg
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            aria-hidden="true"
        >
            <path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" />
            <circle cx="12" cy="7" r="4" />
        </svg>
    }
}

#[component]
pub fn IconBookmark(#[prop(default = 20)] size: u32) -> impl IntoView {
    view! {
        <svg
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            aria-hidden="true"
        >
            <path d="M19 21 12 16 5 21V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z" />
        </svg>
    }
}

/// Renders `children` once the catalogue has loaded, and says so plainly while
/// it has not.
///
/// This is the app's one `Suspense` boundary: the catalogue is a blocking
/// resource, so on a full page load it has already resolved by the time the
/// HTML is sent, and reading it inside a boundary is what keeps hydration
/// honest.
#[component]
pub fn WithCatalog(children: ChildrenFn) -> impl IntoView {
    let catalog = Catalog::from_context();

    view! {
        <Suspense fallback=|| {
            view! {
                <div class="px-[18px] py-12 text-[13px] text-ink/55">
                    "Loading the catalogue…"
                </div>
            }
        }>
            {move || match catalog.status() {
                Status::Ready => children().into_any(),
                Status::Loading => ().into_any(),
                Status::Failed(err) => {
                    view! {
                        <div class="px-[18px] py-12">
                            <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">
                                "The catalogue is unavailable."
                            </div>
                            <p class="mt-2 text-[13px] leading-[1.6] text-ink/62">
                                "The catalogue could not be read from the database."
                            </p>
                            <p class="mt-2 font-mono text-[11px] break-all text-ink/45">{err}</p>
                        </div>
                    }
                        .into_any()
                }
            }}
        </Suspense>
    }
}
