//! Client-side shop state: the bag, the saved list, the search controls and
//! the toast. One struct in context so every screen reads the same signals.
//!
//! None of this touches the server — the bag lives in the tab until checkout
//! exists. Product rows come from [`crate::catalog::Catalog`]; this module
//! only ever holds product ids.

use leptos::prelude::*;

use crate::catalog::Product;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Catalogue,
    PriceLow,
    PriceHigh,
}

impl Sort {
    /// The sort control cycles rather than opening a menu.
    pub fn next(self) -> Self {
        match self {
            Sort::Catalogue => Sort::PriceLow,
            Sort::PriceLow => Sort::PriceHigh,
            Sort::PriceHigh => Sort::Catalogue,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Sort::Catalogue => "Catalogue order",
            Sort::PriceLow => "Price ↑",
            Sort::PriceHigh => "Price ↓",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Shop {
    /// (product id, quantity), in the order things were added.
    cart: RwSignal<Vec<(i32, u32)>>,
    saved: RwSignal<Vec<i32>>,
    toast: RwSignal<Option<String>>,
    /// Bumped on every flash so a stale timer cannot clear a newer toast.
    toast_gen: RwSignal<u32>,
    pub query: RwSignal<String>,
    pub filters: RwSignal<Vec<String>>,
    pub sort: RwSignal<Sort>,
}

impl Default for Shop {
    fn default() -> Self {
        Self::new()
    }
}

impl Shop {
    pub fn new() -> Self {
        Self {
            cart: RwSignal::new(Vec::new()),
            saved: RwSignal::new(Vec::new()),
            toast: RwSignal::new(None),
            toast_gen: RwSignal::new(0),
            query: RwSignal::new(String::new()),
            filters: RwSignal::new(Vec::new()),
            sort: RwSignal::new(Sort::Catalogue),
        }
    }

    pub fn from_context() -> Self {
        expect_context()
    }

    // ── bag ────────────────────────────────────────────────────────────────

    pub fn count(&self) -> u32 {
        self.cart.with(|c| c.iter().map(|(_, q)| q).sum())
    }

    /// Bag total in cents, priced from the catalogue rows passed in.
    pub fn subtotal(&self, products: &[Product]) -> i32 {
        self.cart.with(|c| {
            c.iter()
                .filter_map(|(id, qty)| {
                    products
                        .iter()
                        .find(|p| p.id == *id)
                        .map(|p| p.price_cents * *qty as i32)
                })
                .sum()
        })
    }

    /// The bag as rows, dropping any line whose product left the catalogue.
    pub fn lines(&self, products: &[Product]) -> Vec<(Product, u32)> {
        self.cart.with(|c| {
            c.iter()
                .filter_map(|(id, qty)| {
                    products
                        .iter()
                        .find(|p| p.id == *id)
                        .map(|p| (p.clone(), *qty))
                })
                .collect()
        })
    }

    /// Add `delta` to a line, dropping it when the quantity reaches zero.
    pub fn bump(&self, id: i32, delta: i32) {
        self.cart.update(|c| {
            match c.iter_mut().find(|(i, _)| *i == id) {
                Some(line) => line.1 = line.1.saturating_add_signed(delta),
                None if delta > 0 => c.push((id, delta as u32)),
                None => {}
            }
            c.retain(|(_, q)| *q > 0);
        });
    }

    pub fn remove(&self, id: i32) {
        self.cart.update(|c| c.retain(|(i, _)| *i != id));
    }

    pub fn add(&self, product: &Product) {
        self.bump(product.id, 1);
        self.flash(format!("{} added", product.title));
    }

    // ── saved list ─────────────────────────────────────────────────────────

    pub fn is_saved(&self, id: i32) -> bool {
        self.saved.with(|s| s.contains(&id))
    }

    pub fn toggle_save(&self, id: i32) {
        self.saved
            .update(|s| match s.iter().position(|i| *i == id) {
                Some(at) => {
                    s.remove(at);
                }
                None => s.push(id),
            });
    }

    // ── toast ──────────────────────────────────────────────────────────────

    pub fn toast(&self) -> Option<String> {
        self.toast.get()
    }

    /// Show a message for the length of the `toast` animation. A later flash
    /// wins: the earlier timer sees a bumped generation and does nothing.
    pub fn flash(&self, msg: String) {
        let generation = self.toast_gen.get_untracked().wrapping_add(1);
        self.toast_gen.set(generation);
        self.toast.set(Some(msg));

        let (toast, toast_gen) = (self.toast, self.toast_gen);
        set_timeout(
            move || {
                if toast_gen.get_untracked() == generation {
                    toast.set(None);
                }
            },
            std::time::Duration::from_millis(2400),
        );
    }

    // ── search controls ────────────────────────────────────────────────────

    pub fn is_filtered_by(&self, chip: &str) -> bool {
        self.filters.with(|f| f.iter().any(|c| c == chip))
    }

    pub fn toggle_filter(&self, chip: &str) {
        self.filters
            .update(|f| match f.iter().position(|c| c == chip) {
                Some(at) => {
                    f.remove(at);
                }
                None => f.push(chip.to_string()),
            });
    }

    pub fn reset_filters(&self) {
        self.filters.update(Vec::clear);
        self.query.set(String::new());
    }

    /// The catalogue filtered by the query and chips, then sorted.
    pub fn results<'a>(&self, products: &'a [Product]) -> Vec<&'a Product> {
        let mut found = self.query.with(|q| {
            self.filters
                .with(|f| crate::catalog::search(products, q, f))
        });
        match self.sort.get() {
            Sort::Catalogue => {}
            Sort::PriceLow => found.sort_by_key(|p| p.price_cents),
            Sort::PriceHigh => found.sort_by_key(|p| std::cmp::Reverse(p.price_cents)),
        }
        found
    }
}
