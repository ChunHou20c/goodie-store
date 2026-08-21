//! Shared pieces: the Lucide icon set the design uses, and the photo slot.

use leptos::prelude::*;

/// A photograph in the Modernist system: black and white, filling its
/// container, captioned bottom-left while there is nothing in it yet.
///
/// The catalogue has no imagery, so this renders the placeholder the design
/// draws. When products carry a URL, this is the one place that changes.
#[component]
pub fn Photo(
    #[prop(optional)] label: &'static str,
    /// Extra classes for the frame — a border, usually.
    #[prop(optional)]
    class: &'static str,
) -> impl IntoView {
    view! {
        <div class=format!("photo flex h-full w-full items-end bg-neutral-200 p-2 {class}")>
            <Show when=move || !label.is_empty()>
                <span class="text-[10px] font-semibold uppercase tracking-[0.12em] text-neutral-600">
                    {label}
                </span>
            </Show>
        </div>
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
