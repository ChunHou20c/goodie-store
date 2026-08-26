//! The bag — one row per (user, product) in Postgres.
//!
//! A bag belongs to an account, so a signed-out visitor has no bag at all and
//! the screens send them to `/login?next=…` instead of adding. Every write goes
//! through [`require_user`](crate::auth::require_user); the hidden button is
//! presentation, the gate is here.
//!
//! Shape follows [`crate::auth::Auth`]: a blocking resource whose source tracks
//! the write actions' versions, so a finished write re-reads the bag without a
//! page load. There is no inventory yet — nothing below reserves or deducts
//! `products.stock`.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::auth::Auth;
use crate::catalog::Product;

/// The per-line ceiling, matching the `check` on `cart_items.quantity`.
pub const MAX_QUANTITY: i32 = 99;

/// A row of `cart_items`. Quantities are `i32` because Postgres `integer` is,
/// and because prices are `i32` cents — no casts on the way to a subtotal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct CartLine {
    pub product_id: i32,
    pub quantity: i32,
}

/// What a requested quantity means: `None` removes the line, `Some(n)` sets it.
/// Free function so the clamping is testable without a database.
pub fn clamp_quantity(requested: i32) -> Option<i32> {
    (requested > 0).then(|| requested.min(MAX_QUANTITY))
}

// ── server functions ───────────────────────────────────────────────────────

/// The signed-in user's bag. Signed out reads as an empty bag rather than an
/// error: the top bar asks for this on every page, and a visitor is not a fault.
#[server(endpoint = "list_cart")]
pub async fn list_cart() -> Result<Vec<CartLine>, ServerFnError> {
    use crate::auth::current_user_from_request;

    let Some(user) = current_user_from_request().await else {
        return Ok(Vec::new());
    };
    let pool = expect_context::<sqlx::PgPool>();

    sqlx::query_as::<_, CartLine>(
        "select product_id, quantity from cart_items \
         where user_id = $1 order by added_at, product_id",
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("could not read the bag: {e}")))
}

#[server(endpoint = "add_to_cart")]
pub async fn add_to_cart(product_id: i32) -> Result<(), ServerFnError> {
    use crate::auth::require_user;

    let user = require_user().await?;
    let pool = expect_context::<sqlx::PgPool>();

    sqlx::query(
        "insert into cart_items (user_id, product_id, quantity) values ($1, $2, 1) \
         on conflict (user_id, product_id) do update \
         set quantity = least(cart_items.quantity + 1, $3), updated_at = now()",
    )
    .bind(user.id)
    .bind(product_id)
    .bind(MAX_QUANTITY)
    .execute(&pool)
    .await
    .map_err(unknown_product)?;

    Ok(())
}

/// Set a line to an exact quantity; anything at or below zero removes it.
#[server(endpoint = "set_cart_quantity")]
pub async fn set_cart_quantity(product_id: i32, quantity: i32) -> Result<(), ServerFnError> {
    use crate::auth::require_user;

    let user = require_user().await?;
    let pool = expect_context::<sqlx::PgPool>();

    let Some(quantity) = clamp_quantity(quantity) else {
        return remove_from_cart(product_id).await;
    };

    sqlx::query(
        "insert into cart_items (user_id, product_id, quantity) values ($1, $2, $3) \
         on conflict (user_id, product_id) do update \
         set quantity = excluded.quantity, updated_at = now()",
    )
    .bind(user.id)
    .bind(product_id)
    .bind(quantity)
    .execute(&pool)
    .await
    .map_err(unknown_product)?;

    Ok(())
}

#[server(endpoint = "remove_from_cart")]
pub async fn remove_from_cart(product_id: i32) -> Result<(), ServerFnError> {
    use crate::auth::require_user;

    let user = require_user().await?;
    let pool = expect_context::<sqlx::PgPool>();

    sqlx::query("delete from cart_items where user_id = $1 and product_id = $2")
        .bind(user.id)
        .bind(product_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("could not update the bag: {e}")))?;

    Ok(())
}

/// The only write failure a caller can provoke is naming a product that is not
/// there, which the foreign key catches. Say that rather than quoting sqlx.
#[cfg(feature = "ssr")]
fn unknown_product(e: sqlx::Error) -> ServerFnError {
    match &e {
        sqlx::Error::Database(db) if db.is_foreign_key_violation() => crate::auth::refuse(
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            "That product is no longer in the catalogue.",
        ),
        // Anything else here really is our fault, so it keeps the 500.
        _ => ServerFnError::new(format!("could not update the bag: {e}")),
    }
}

// ── reactive context ───────────────────────────────────────────────────────

/// The bag as the screens read it, plus the actions that change it.
#[derive(Clone, Copy)]
pub struct Cart {
    lines: Resource<Result<Vec<CartLine>, ServerFnError>>,
    /// The product an add is currently announcing, set at the click and taken
    /// by whoever reports it. It lives here, alongside the action, because the
    /// screen that dispatched can be torn down and rebuilt before the response
    /// lands — a watermark held by the button would go with it.
    last_added: RwSignal<Option<i32>>,
    pub add: ServerAction<AddToCart>,
    pub set_quantity: ServerAction<SetCartQuantity>,
    pub remove: ServerAction<RemoveFromCart>,
}

impl Cart {
    /// Takes `Auth` rather than reading it from context so the ordering in
    /// [`crate::app::App`] is a compile error to get wrong.
    pub fn load(auth: Auth) -> Self {
        let add = ServerAction::<AddToCart>::new();
        let set_quantity = ServerAction::<SetCartQuantity>::new();
        let remove = ServerAction::<RemoveFromCart>::new();

        // The three writes, plus who is signed in: signing out has to empty the
        // bag on screen, not only in the database.
        let version = move || {
            (
                auth.version(),
                add.version().get(),
                set_quantity.version().get(),
                remove.version().get(),
            )
        };
        let lines = Resource::new_blocking(version, |_| list_cart());

        Self {
            lines,
            last_added: RwSignal::new(None),
            add,
            set_quantity,
            remove,
        }
    }

    pub fn from_context() -> Self {
        expect_context()
    }

    /// Read the loaded rows without cloning. A failed read is an empty bag —
    /// the chrome should never break because the bag query did.
    pub fn with<T>(&self, f: impl FnOnce(&[CartLine]) -> T) -> T {
        self.lines.with(|loaded| match loaded {
            Some(Ok(lines)) => f(lines),
            _ => f(&[]),
        })
    }

    pub fn count(&self) -> i32 {
        self.with(|lines| lines.iter().map(|l| l.quantity).sum())
    }

    pub fn is_empty(&self) -> bool {
        self.with(<[CartLine]>::is_empty)
    }

    /// Bag total in cents, priced from the catalogue rows passed in.
    pub fn subtotal(&self, products: &[Product]) -> i32 {
        self.with(|lines| {
            lines
                .iter()
                .filter_map(|line| {
                    products
                        .iter()
                        .find(|p| p.id == line.product_id)
                        .map(|p| p.price_cents * line.quantity)
                })
                .sum()
        })
    }

    /// The bag as rows, dropping any line whose product left the catalogue.
    pub fn rows(&self, products: &[Product]) -> Vec<(Product, i32)> {
        self.with(|lines| {
            lines
                .iter()
                .filter_map(|line| {
                    products
                        .iter()
                        .find(|p| p.id == line.product_id)
                        .map(|p| (p.clone(), line.quantity))
                })
                .collect()
        })
    }

    /// True while any write is in flight; the steppers disable on it so a burst
    /// of taps cannot race the refetch.
    pub fn pending(&self) -> bool {
        self.add.pending().get() || self.set_quantity.pending().get() || self.remove.pending().get()
    }

    /// The message from whichever write failed last, for `FormError`.
    pub fn last_error(&self) -> Option<String> {
        fn failed<T: 'static + Send + Sync>(
            value: Option<Result<T, ServerFnError>>,
        ) -> Option<String> {
            match value {
                Some(Err(e)) => Some(e.to_string()),
                _ => None,
            }
        }

        failed(self.add.value().get())
            .or_else(|| failed(self.set_quantity.value().get()))
            .or_else(|| failed(self.remove.value().get()))
    }

    // ── writes ─────────────────────────────────────────────────────────────

    pub fn add_item(&self, product_id: i32) {
        self.last_added.set(Some(product_id));
        self.add.dispatch(AddToCart { product_id });
    }

    /// The product of the add that just settled, once. Untracked so the caller
    /// can clear it from inside the effect that reports it.
    pub fn take_last_added(&self) -> Option<i32> {
        let id = self.last_added.get_untracked();
        if id.is_some() {
            self.last_added.set(None);
        }
        id
    }

    /// Callers pass the quantity they want, computed from the row they already
    /// rendered, so a click handler never has to read the resource.
    pub fn set(&self, product_id: i32, quantity: i32) {
        self.set_quantity.dispatch(SetCartQuantity {
            product_id,
            quantity,
        });
    }

    pub fn remove_item(&self, product_id: i32) {
        self.remove.dispatch(RemoveFromCart { product_id });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product(id: i32, price_cents: i32) -> Product {
        Product {
            id,
            slug: format!("p-{id}"),
            title: format!("Product {id}"),
            category: "beauty".into(),
            description: "A thing.".into(),
            note: None,
            price_cents,
            rating: None,
            available: 10,
            availability: "In Stock".into(),
            brand: None,
            sku: None,
            weight_grams: None,
            width_mm: None,
            height_mm: None,
            depth_mm: None,
            warranty: None,
            shipping: None,
            return_policy: None,
            min_order: None,
            thumbnail_url: None,
            tags: Vec::new(),
        }
    }

    fn line(product_id: i32, quantity: i32) -> CartLine {
        CartLine {
            product_id,
            quantity,
        }
    }

    // The reading helpers, as free functions over the same slices `Cart` holds,
    // so they can be exercised without a reactive runtime.
    fn subtotal(lines: &[CartLine], products: &[Product]) -> i32 {
        lines
            .iter()
            .filter_map(|l| {
                products
                    .iter()
                    .find(|p| p.id == l.product_id)
                    .map(|p| p.price_cents * l.quantity)
            })
            .sum()
    }

    #[test]
    fn zero_and_below_remove_the_line() {
        assert_eq!(clamp_quantity(0), None);
        assert_eq!(clamp_quantity(-3), None);
    }

    #[test]
    fn quantity_is_capped_not_rejected() {
        assert_eq!(clamp_quantity(1), Some(1));
        assert_eq!(clamp_quantity(MAX_QUANTITY), Some(MAX_QUANTITY));
        assert_eq!(clamp_quantity(MAX_QUANTITY + 1), Some(MAX_QUANTITY));
        assert_eq!(clamp_quantity(i32::MAX), Some(MAX_QUANTITY));
    }

    #[test]
    fn subtotal_multiplies_each_line() {
        let products = [product(1, 999), product(2, 12999)];
        let lines = [line(1, 3), line(2, 1)];
        assert_eq!(subtotal(&lines, &products), 999 * 3 + 12999);
    }

    #[test]
    fn a_line_whose_product_left_the_catalogue_is_dropped() {
        let products = [product(1, 999)];
        let lines = [line(1, 2), line(404, 5)];
        // Priced as if the missing line were not there, rather than at zero or
        // a panic — the bag screen drops the row for the same reason.
        assert_eq!(subtotal(&lines, &products), 1998);
    }

    #[test]
    fn an_empty_bag_is_worth_nothing() {
        assert_eq!(subtotal(&[], &[product(1, 999)]), 0);
    }
}
