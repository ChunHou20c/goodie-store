//! Kessel — the shop shell: top bar, screen, sticky action, bottom nav.
//!
//! Implements `Kessel Shop.dc.html` from the "Mobile shopping app design"
//! Claude Design project, on the Modernist design system. The prototype
//! switches screens in local state; on the web each screen is a real route,
//! so a product is linkable and the back button behaves.

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::hooks::use_location;
use leptos_router::{ParamSegment, StaticSegment};

use crate::auth::Auth;
use crate::catalog::{money, Catalog, Product};
use crate::screens::{
    AdminScreen, BagScreen, HomeScreen, LoginScreen, ProductScreen, SearchScreen,
};
use crate::shop::Shop;
use crate::ui::{IconBack, IconBag, IconBookmark, IconSearch, IconUser};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="theme-color" content="#f3f2f2" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// Which of the four screens the current path is on. The chrome — back label,
/// sticky action, active tab — is derived from this rather than from state.
#[derive(Clone, PartialEq)]
enum Screen {
    Home,
    Search,
    /// A product page, by slug; the row itself comes from the catalogue.
    Product(String),
    Bag,
    Login,
    Admin,
    Unknown,
}

fn screen_of(path: &str) -> Screen {
    match path.trim_end_matches('/') {
        "" => Screen::Home,
        "/search" => Screen::Search,
        "/bag" => Screen::Bag,
        "/login" => Screen::Login,
        "/admin" => Screen::Admin,
        other => other
            .strip_prefix("/p/")
            .map(|slug| Screen::Product(slug.to_string()))
            .unwrap_or(Screen::Unknown),
    }
}

fn use_screen() -> impl Fn() -> Screen + Copy {
    let path = use_location().pathname;
    move || screen_of(&path.get())
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_context(Shop::new());
    // One blocking resource for the catalogue: resolved during SSR, serialized
    // into the response, and read from there on the client.
    provide_context(Catalog::load());
    provide_context(Auth::load());

    view! {
        <Stylesheet id="leptos" href="/pkg/goodie-never-deliver.css" />
        <Title text="Kessel — an electronics store that reads like a magazine" />

        <Router>
            <div class="mx-auto flex min-h-dvh w-full max-w-[430px] flex-col bg-ground sm:border-x-2 sm:border-ink/40">
                <TopBar />
                <main class="flex-1">
                    <Routes fallback=NotFound>
                        <Route path=StaticSegment("") view=HomeScreen />
                        <Route path=StaticSegment("search") view=SearchScreen />
                        <Route path=(StaticSegment("p"), ParamSegment("id")) view=ProductScreen />
                        <Route path=StaticSegment("bag") view=BagScreen />
                        <Route path=StaticSegment("login") view=LoginScreen />
                        <Route path=StaticSegment("admin") view=AdminScreen />
                    </Routes>
                </main>
                <div class="sticky bottom-0 z-20 bg-ground">
                    <Toast />
                    <StickyAction />
                    <BottomNav />
                </div>
            </div>
        </Router>
    }
}

#[component]
fn TopBar() -> impl IntoView {
    let shop = Shop::from_context();
    let auth = Auth::from_context();
    let screen = use_screen();

    view! {
        <header class="sticky top-0 z-20 flex items-center justify-between border-b-2 border-ink/40 bg-ground px-[18px] pt-3.5 pb-3">
            {move || match screen() {
                Screen::Home => {
                    view! {
                        <div class="font-heading text-[19px] font-extrabold tracking-[0.16em]">
                            "KESSEL"
                        </div>
                    }
                        .into_any()
                }
                other => {
                    let label = if matches!(other, Screen::Product(_)) { "Index" } else { "Back" };
                    view! {
                        <A
                            href="/"
                            {..}
                            class="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.12em] text-ink no-underline hover:text-accent"
                        >
                            <IconBack />
                            {label}
                        </A>
                    }
                        .into_any()
                }
            }} <div class="flex items-center gap-3.5">
                // Signed out this is the Lucide user glyph; signed in it becomes
                // the same accent square the bag badge uses, carrying an initial.
                <Suspense fallback=|| {
                    view! {
                        <A
                            href="/login"
                            {..}
                            class="flex text-ink hover:text-accent"
                            aria-label="Sign in"
                        >
                            <IconUser />
                        </A>
                    }
                }>
                    {move || {
                        match auth.with(|user| user.map(|u| u.initial())) {
                            Some(initial) => {
                                view! {
                                    <A
                                        href="/login"
                                        {..}
                                        class="flex text-ink no-underline hover:text-accent"
                                        aria-label="Account"
                                    >
                                        <span class="flex h-[18px] min-w-[18px] items-center justify-center bg-accent px-1 text-[11px] font-extrabold text-ground">
                                            {initial}
                                        </span>
                                    </A>
                                }
                                    .into_any()
                            }
                            None => {
                                view! {
                                    <A
                                        href="/login"
                                        {..}
                                        class="flex text-ink hover:text-accent"
                                        aria-label="Sign in"
                                    >
                                        <IconUser />
                                    </A>
                                }
                                    .into_any()
                            }
                        }
                    }}
                </Suspense>
                <A href="/search" {..} class="flex text-ink hover:text-accent" aria-label="Search">
                    <IconSearch />
                </A>
                <A
                    href="/bag"
                    {..}
                    class="flex items-center gap-[5px] text-ink hover:text-accent"
                    aria-label="Bag"
                >
                    <IconBag />
                    <Show when=move || { shop.count() > 0 }>
                        <span class="flex h-[18px] min-w-[18px] items-center justify-center bg-accent px-1 text-[11px] font-extrabold text-ground">
                            {move || shop.count()}
                        </span>
                    </Show>
                </A>
            </div>
        </header>
    }
}

/// The bar above the tabs: add-to-bag on a product, checkout in the bag.
#[component]
fn StickyAction() -> impl IntoView {
    let shop = Shop::from_context();
    let screen = use_screen();

    let catalog = Catalog::from_context();

    let action = move || match screen() {
        Screen::Product(slug) => match catalog.by_slug(&slug) {
            Some(product) => view! { <ProductActions product /> }.into_any(),
            None => ().into_any(),
        },
        Screen::Bag => view! {
            <Show when=move || { shop.count() > 0 }>
                <div class="border-t-2 border-ink/40 bg-ground px-[18px] py-3">
                    <button
                        class="btn btn-primary w-full justify-between px-4 py-[15px]"
                        on:click=move |_| shop.flash("Payment step — coming next".to_string())
                    >
                        "Checkout"
                        <span>
                            {move || catalog.with(|products| money(shop.subtotal(products)))}
                        </span>
                    </button>
                </div>
            </Show>
        }
        .into_any(),
        _ => ().into_any(),
    };

    // Both arms price themselves from the catalogue, so they read it inside a
    // boundary like every other consumer.
    view! { <Suspense fallback=|| ()>{action}</Suspense> }
}

#[component]
fn ProductActions(product: Product) -> impl IntoView {
    let shop = Shop::from_context();
    let id = product.id;
    let price = product.price_label();
    let saved = move || shop.is_saved(id);

    view! {
        <div class="flex gap-2.5 border-t-2 border-ink/40 bg-ground px-[18px] py-3">
            <button
                class=move || {
                    if saved() {
                        "flex w-12 flex-none cursor-pointer items-center justify-center border-2 border-ink bg-ink text-ground"
                    } else {
                        "flex w-12 flex-none cursor-pointer items-center justify-center border-2 border-ink bg-transparent text-ink hover:bg-ink/7"
                    }
                }
                aria-pressed=move || if saved() { "true" } else { "false" }
                aria-label="Save for later"
                on:click=move |_| shop.toggle_save(id)
            >
                <IconBookmark />
            </button>
            <button
                class="btn btn-primary flex-1 justify-between px-4 py-[15px]"
                on:click=move |_| shop.add(&product)
            >
                "Add to bag"
                <span>{price}</span>
            </button>
        </div>
    }
}

#[component]
fn BottomNav() -> impl IntoView {
    let shop = Shop::from_context();
    let screen = use_screen();

    view! {
        <nav class="grid grid-cols-3 border-t-2 border-ink/40 bg-ground">
            {move || {
                let here = screen();
                let count = shop.count();
                let bag_sub = if count == 1 {
                    "1 item".to_string()
                } else {
                    format!("{count} items")
                };
                let tabs = [
                    (
                        "Index",
                        "Issue 14".to_string(),
                        "/",
                        matches!(here, Screen::Home | Screen::Product(_)),
                    ),
                    ("Search", "Filter".to_string(), "/search", here == Screen::Search),
                    ("Bag", bag_sub, "/bag", here == Screen::Bag),
                ];
                tabs.into_iter()
                    .map(|(label, sub, to, active)| {
                        let class = if active {
                            "flex flex-col items-center gap-[5px] border-r border-ink/14 bg-ink px-0 pt-[11px] pb-3 text-ground no-underline"
                        } else {
                            "flex flex-col items-center gap-[5px] border-r border-ink/14 bg-transparent px-0 pt-[11px] pb-3 text-ink no-underline hover:bg-accent-100"
                        };
                        // <A> sets aria-current="page" itself when the href matches.
                        view! {
                            <A href={to} {..} class=class>
                                <span class="text-[11px] font-extrabold uppercase tracking-[0.14em]">
                                    {label}
                                </span>
                                <span class="text-[10px] tracking-[0.06em] opacity-70">{sub}</span>
                            </A>
                        }
                    })
                    .collect_view()
            }}
        </nav>
    }
}

#[component]
fn Toast() -> impl IntoView {
    let shop = Shop::from_context();

    view! {
        {move || {
            shop.toast()
                .map(|msg| {
                    view! {
                        // The design pins this 96px off the bottom, which lands it on top of
                        // the sticky action; anchoring above the footer keeps the same look
                        // without covering the button the toast is reporting on.
                        <div class="pointer-events-none absolute inset-x-0 bottom-full z-30 mb-3 px-[18px]">
                            <div class="pointer-events-auto">
                                <div class="animate-toast flex items-center justify-between gap-3 bg-ink px-[15px] py-[13px] text-ground shadow-lg">
                                    <span class="text-[13px] font-semibold">{msg}</span>
                                    <A
                                        href="/bag"
                                        {..}
                                        class="text-[11.5px] font-extrabold uppercase tracking-[0.1em] text-accent-400 no-underline"
                                    >
                                        "View bag"
                                    </A>
                                </div>
                            </div>
                        </div>
                    }
                })
        }}
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="px-[18px] py-12">
            <div class="font-heading text-xl font-extrabold tracking-[-0.015em]">
                "That page is not in this issue."
            </div>
            <p class="mt-2 mb-4 text-[13px] leading-[1.6] text-ink/62">
                "Every object lives in the index — start there."
            </p>
            <A href="/" {..} class="btn btn-primary no-underline">
                "Back to Issue 14"
            </A>
        </div>
    }
}
