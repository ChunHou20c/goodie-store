//! Client-side shop state: the bag, the saved list, the search controls and
//! the toast. One struct in context so every screen reads the same signals.

use leptos::prelude::*;

use crate::catalog::{self, Product};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Index,
    PriceLow,
    PriceHigh,
}

impl Sort {
    /// The sort control cycles rather than opening a menu.
    pub fn next(self) -> Self {
        match self {
            Sort::Index => Sort::PriceLow,
            Sort::PriceLow => Sort::PriceHigh,
            Sort::PriceHigh => Sort::Index,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Sort::Index => "Issue order",
            Sort::PriceLow => "Price ↑",
            Sort::PriceHigh => "Price ↓",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Shop {
    /// (product id, quantity), in the order things were added.
    cart: RwSignal<Vec<(&'static str, u32)>>,
    saved: RwSignal<Vec<&'static str>>,
    toast: RwSignal<Option<String>>,
    /// Bumped on every flash so a stale timer cannot clear a newer toast.
    toast_gen: RwSignal<u32>,
    pub query: RwSignal<String>,
    pub filters: RwSignal<Vec<&'static str>>,
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
            sort: RwSignal::new(Sort::Index),
        }
    }

    pub fn from_context() -> Self {
        expect_context()
    }

    // ── bag ────────────────────────────────────────────────────────────────

    pub fn count(&self) -> u32 {
        self.cart.with(|c| c.iter().map(|(_, q)| q).sum())
    }

    pub fn subtotal(&self) -> u32 {
        self.cart.with(|c| {
            c.iter()
                .filter_map(|(id, q)| catalog::find(id).map(|p| p.price * q))
                .sum()
        })
    }

    pub fn lines(&self) -> Vec<(&'static Product, u32)> {
        self.cart.with(|c| {
            c.iter()
                .filter_map(|(id, q)| catalog::find(id).map(|p| (p, *q)))
                .collect()
        })
    }

    /// Add `delta` to a line, dropping it when the quantity reaches zero.
    pub fn bump(&self, id: &'static str, delta: i32) {
        self.cart.update(|c| {
            match c.iter_mut().find(|(i, _)| *i == id) {
                Some(line) => line.1 = line.1.saturating_add_signed(delta),
                None if delta > 0 => c.push((id, delta as u32)),
                None => {}
            }
            c.retain(|(_, q)| *q > 0);
        });
    }

    pub fn remove(&self, id: &'static str) {
        self.cart.update(|c| c.retain(|(i, _)| *i != id));
    }

    pub fn add(&self, p: &'static Product) {
        self.bump(p.id, 1);
        self.flash(format!("{} added", p.name));
    }

    // ── saved list ─────────────────────────────────────────────────────────

    pub fn is_saved(&self, id: &'static str) -> bool {
        self.saved.with(|s| s.contains(&id))
    }

    pub fn toggle_save(&self, id: &'static str) {
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
        let gen = self.toast_gen.get_untracked().wrapping_add(1);
        self.toast_gen.set(gen);
        self.toast.set(Some(msg));

        let (toast, toast_gen) = (self.toast, self.toast_gen);
        set_timeout(
            move || {
                if toast_gen.get_untracked() == gen {
                    toast.set(None);
                }
            },
            std::time::Duration::from_millis(2400),
        );
    }

    // ── search controls ────────────────────────────────────────────────────

    pub fn toggle_filter(&self, chip: &'static str) {
        self.filters
            .update(|f| match f.iter().position(|c| *c == chip) {
                Some(at) => {
                    f.remove(at);
                }
                None => f.push(chip),
            });
    }

    pub fn reset_filters(&self) {
        self.filters.update(Vec::clear);
        self.query.set(String::new());
    }

    /// Filtered, then sorted by the current sort.
    pub fn results(&self) -> Vec<&'static Product> {
        let mut found = self
            .query
            .with(|q| self.filters.with(|f| catalog::search(q, f)));
        match self.sort.get() {
            Sort::Index => {}
            Sort::PriceLow => found.sort_by_key(|p| p.price),
            Sort::PriceHigh => found.sort_by_key(|p| std::cmp::Reverse(p.price)),
        }
        found
    }
}
