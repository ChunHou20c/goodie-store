//! Checkout — turning a bag into a claim on stock, and a claim into an order.
//!
//! Pressing Checkout creates a `pending` reservation, raises
//! `inventory.reserved` and empties the bag. Paying takes the goods out of the
//! building (`on_hand` and `reserved` both fall) and writes an order. Walking
//! away lets it expire, which releases the stock but does **not** give the bag
//! back — a lapsed checkout starts over.
//!
//! A shopper has at most one pending reservation, enforced by a partial unique
//! index, so pressing Checkout twice returns the reservation they already have
//! rather than opening a second claim.
//!
//! There is no scheduler in this app, so expiry is a sweep run at the top of
//! every write here, inside the same transaction.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::auth::Auth;
use crate::catalog::money;

/// How long a checkout holds stock before the sweep releases it.
pub const RESERVATION_TTL_MINUTES: i64 = 15;

/// One line of a reservation or an order, as the screens want to read it.
///
/// `title` is carried rather than looked up so a receipt reads correctly
/// regardless of what the catalogue does afterwards.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Line {
    pub product_id: i32,
    pub title: String,
    pub quantity: i32,
    pub unit_price_cents: i32,
}

impl Line {
    pub fn subtotal_cents(&self) -> i32 {
        self.unit_price_cents * self.quantity
    }

    pub fn subtotal_label(&self) -> String {
        money(self.subtotal_cents())
    }
}

/// What the whole basket comes to.
pub fn total_cents(lines: &[Line]) -> i32 {
    lines.iter().map(Line::subtotal_cents).sum()
}

/// A pending reservation, for the payment screen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReservationView {
    pub id: i64,
    pub lines: Vec<Line>,
    /// Seconds left before the sweep releases it; never negative.
    pub expires_in_secs: i32,
}

impl ReservationView {
    pub fn total_cents(&self) -> i32 {
        total_cents(&self.lines)
    }

    pub fn total_label(&self) -> String {
        money(self.total_cents())
    }

    /// How long is left, in words. Rendered once when the page loads — there is
    /// no live countdown.
    pub fn expires_label(&self) -> String {
        expires_label(self.expires_in_secs)
    }
}

/// Shared so it can be tested without a database or a reactive runtime.
pub fn expires_label(secs: i32) -> String {
    match secs {
        s if s <= 0 => "expired".to_string(),
        s if s < 60 => "expires in under a minute".to_string(),
        s if s < 120 => "expires in 1 minute".to_string(),
        s => format!("expires in {} minutes", s / 60),
    }
}

/// A placed order, for the receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderView {
    pub id: i64,
    pub lines: Vec<Line>,
    pub total_cents: i32,
}

impl OrderView {
    pub fn total_label(&self) -> String {
        money(self.total_cents)
    }
}

// ── server-only helpers ────────────────────────────────────────────────────

#[cfg(feature = "ssr")]
mod ssr {
    use super::*;

    use sqlx::PgPool;

    /// Release every pending reservation that has run out of time, and mark it
    /// expired.
    ///
    /// Runs on the pool, **not** inside the caller's transaction, and so commits
    /// on its own. That matters: the calls below can fail after sweeping — an
    /// expired checkout, an oversold line — and rolling their work back must not
    /// also un-expire reservations that genuinely ran out of time.
    ///
    /// One statement on purpose: the `update … returning` claims the rows, so
    /// two requests sweeping at the same moment cannot both release the same
    /// reservation and drive `reserved` below zero.
    pub async fn sweep_expired(pool: &PgPool) -> Result<(), ServerFnError> {
        sqlx::query(
            "with expired as ( \
                 update reservations set status = 'expired', settled_at = now() \
                 where status = 'pending' and expires_at <= now() \
                 returning id \
             ), lines as ( \
                 select ri.product_id, sum(ri.quantity) as qty \
                 from reservation_items ri join expired e on e.id = ri.reservation_id \
                 group by ri.product_id \
             ) \
             update inventory i \
             set reserved = i.reserved - l.qty, updated_at = now() \
             from lines l where i.product_id = l.product_id",
        )
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("could not release expired holds: {e}")))?;

        Ok(())
    }

    /// The lines of a reservation, priced as they were when it was taken.
    /// Generic over the executor so it reads the same from a pool or from
    /// inside a transaction.
    pub async fn reservation_lines<'e, E>(
        executor: E,
        reservation_id: i64,
    ) -> Result<Vec<Line>, ServerFnError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, Line>(
            "select ri.product_id, p.title, ri.quantity, ri.unit_price_cents \
             from reservation_items ri join products p on p.id = ri.product_id \
             where ri.reservation_id = $1 order by ri.product_id",
        )
        .bind(reservation_id)
        .fetch_all(executor)
        .await
        .map_err(|e| ServerFnError::new(format!("could not read the checkout: {e}")))
    }
}

// ── server functions ───────────────────────────────────────────────────────

/// Take a claim on the stock in the bag, and empty it.
///
/// Returns the reservation to pay for. Pressing this again while one is pending
/// returns that same one — the button is safe to press twice, and the screen
/// guides you back to the checkout you already started.
#[server(endpoint = "start_checkout")]
pub async fn start_checkout() -> Result<i64, ServerFnError> {
    use self::ssr::*;
    use crate::auth::{refuse, require_user, StatusCode};

    let user = require_user().await?;
    let pool = expect_context::<sqlx::PgPool>();
    sweep_expired(&pool).await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("checkout failed to start: {e}")))?;

    // Already holding one? Send them back to it rather than claiming more.
    let existing: Option<(i64,)> =
        sqlx::query_as("select id from reservations where user_id = $1 and status = 'pending'")
            .bind(user.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(format!("could not read your checkout: {e}")))?;
    if let Some((id,)) = existing {
        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(format!("checkout failed to finish: {e}")))?;
        return Ok(id);
    }

    // The bag, priced now — the reservation keeps this price even if the
    // catalogue changes before payment.
    let lines = sqlx::query_as::<_, Line>(
        "select c.product_id, p.title, c.quantity, p.price_cents as unit_price_cents \
         from cart_items c join products p on p.id = c.product_id \
         where c.user_id = $1 order by c.added_at, c.product_id",
    )
    .bind(user.id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("could not read your bag: {e}")))?;

    if lines.is_empty() {
        return Err(refuse(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Your bag is empty.",
        ));
    }

    // Check and claim in one statement per line. Two shoppers racing for the
    // last unit cannot both win: the row is locked for the update, and the
    // `available >=` guard is evaluated against the locked row.
    for line in &lines {
        let claimed = sqlx::query(
            "update inventory set reserved = reserved + $2, updated_at = now() \
             where product_id = $1 and available >= $2",
        )
        .bind(line.product_id)
        .bind(line.quantity)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("could not hold stock: {e}")))?;

        if claimed.rows_affected() == 0 {
            // Rolling back releases whatever earlier lines already claimed.
            let left: Option<(i32,)> =
                sqlx::query_as("select available from inventory where product_id = $1")
                    .bind(line.product_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .ok()
                    .flatten();
            let left = left.map(|(n,)| n).unwrap_or(0);
            return Err(refuse(
                StatusCode::CONFLICT,
                format!("Only {left} left of {}.", line.title),
            ));
        }
    }

    let (reservation_id,): (i64,) = sqlx::query_as(
        "insert into reservations (user_id, expires_at) \
         values ($1, now() + make_interval(mins => $2)) returning id",
    )
    .bind(user.id)
    // `make_interval(mins => …)` takes an int; only `secs` is double precision.
    .bind(RESERVATION_TTL_MINUTES as i32)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("could not open the checkout: {e}")))?;

    for line in &lines {
        sqlx::query(
            "insert into reservation_items (reservation_id, product_id, quantity, unit_price_cents) \
             values ($1, $2, $3, $4)",
        )
        .bind(reservation_id)
        .bind(line.product_id)
        .bind(line.quantity)
        .bind(line.unit_price_cents)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("could not open the checkout: {e}")))?;
    }

    // The bag becomes the reservation; expiry does not give it back.
    sqlx::query("delete from cart_items where user_id = $1")
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("could not empty the bag: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("checkout failed to finish: {e}")))?;

    Ok(reservation_id)
}

/// Stand in for a payment provider: settle the reservation and write the order.
///
/// Returns the order id. Paying an already-paid reservation returns the order
/// that exists rather than failing — a double click should land on the receipt,
/// and the `orders.reservation_id` unique constraint means it cannot do worse.
#[server(endpoint = "pay")]
pub async fn pay(reservation_id: i64) -> Result<i64, ServerFnError> {
    use self::ssr::*;
    use crate::auth::{refuse, require_user, StatusCode};

    let user = require_user().await?;
    let pool = expect_context::<sqlx::PgPool>();
    sweep_expired(&pool).await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("payment failed to start: {e}")))?;

    // Scoped to the payer, and locked for the rest of this transaction.
    let found: Option<(String,)> = sqlx::query_as(
        "select status::text from reservations where id = $1 and user_id = $2 for update",
    )
    .bind(reservation_id)
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("could not read the checkout: {e}")))?;

    // Someone else's reservation reads the same as one that never existed.
    let Some((status,)) = found else {
        return Err(refuse(StatusCode::NOT_FOUND, "That checkout is not open."));
    };

    match status.as_str() {
        "fulfilled" => {
            let (order_id,): (i64,) =
                sqlx::query_as("select id from orders where reservation_id = $1")
                    .bind(reservation_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| ServerFnError::new(format!("could not find the order: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| ServerFnError::new(format!("payment failed to finish: {e}")))?;
            return Ok(order_id);
        }
        "expired" => {
            return Err(refuse(
                StatusCode::CONFLICT,
                "That checkout expired and the items went back on the shelf.",
            ));
        }
        _ => {}
    }

    let lines = reservation_lines(&mut *tx, reservation_id).await?;

    // The goods leave the building. Both columns fall together, so
    // `reserved <= on_hand` and `on_hand >= 0` still hold afterwards.
    for line in &lines {
        sqlx::query(
            "update inventory \
             set on_hand = on_hand - $2, reserved = reserved - $2, updated_at = now() \
             where product_id = $1",
        )
        .bind(line.product_id)
        .bind(line.quantity)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("could not ship stock: {e}")))?;
    }

    let (order_id,): (i64,) = sqlx::query_as(
        "insert into orders (user_id, reservation_id, total_cents) values ($1, $2, $3) \
         returning id",
    )
    .bind(user.id)
    .bind(reservation_id)
    .bind(total_cents(&lines))
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("could not place the order: {e}")))?;

    for line in &lines {
        sqlx::query(
            "insert into order_items (order_id, product_id, title, quantity, unit_price_cents) \
             values ($1, $2, $3, $4, $5)",
        )
        .bind(order_id)
        .bind(line.product_id)
        .bind(&line.title)
        .bind(line.quantity)
        .bind(line.unit_price_cents)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("could not place the order: {e}")))?;
    }

    sqlx::query("update reservations set status = 'fulfilled', settled_at = now() where id = $1")
        .bind(reservation_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("could not close the checkout: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("payment failed to finish: {e}")))?;

    Ok(order_id)
}

/// The checkout waiting to be paid for, if there is one.
///
/// Signed out reads as "nothing pending" rather than an error: the bag screen
/// asks for this on a page a visitor can see.
#[server(endpoint = "current_reservation")]
pub async fn current_reservation() -> Result<Option<ReservationView>, ServerFnError> {
    use self::ssr::*;
    use crate::auth::current_user_from_request;

    let Some(user) = current_user_from_request().await else {
        return Ok(None);
    };
    let pool = expect_context::<sqlx::PgPool>();
    sweep_expired(&pool).await?;

    // `timestamptz` would need a chrono/time feature on sqlx that this crate
    // does not carry, and the screen only needs the remaining time anyway.
    let found: Option<(i64, i32)> = sqlx::query_as(
        "select id, greatest(0, extract(epoch from (expires_at - now()))::int) \
         from reservations where user_id = $1 and status = 'pending'",
    )
    .bind(user.id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("could not read your checkout: {e}")))?;

    let view = match found {
        Some((id, expires_in_secs)) => {
            let lines = reservation_lines(&pool, id).await?;
            Some(ReservationView {
                id,
                lines,
                expires_in_secs,
            })
        }
        None => None,
    };

    Ok(view)
}

/// A placed order, scoped to the signed-in buyer so `/checkout?order=N`
/// survives a reload without becoming a way to read someone else's receipt.
#[server(endpoint = "get_order")]
pub async fn get_order(order_id: i64) -> Result<Option<OrderView>, ServerFnError> {
    use crate::auth::current_user_from_request;

    let Some(user) = current_user_from_request().await else {
        return Ok(None);
    };
    let pool = expect_context::<sqlx::PgPool>();

    let found: Option<(i64, i32)> =
        sqlx::query_as("select id, total_cents from orders where id = $1 and user_id = $2")
            .bind(order_id)
            .bind(user.id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("could not read the order: {e}")))?;

    let Some((id, total_cents)) = found else {
        return Ok(None);
    };

    let lines = sqlx::query_as::<_, Line>(
        "select product_id, title, quantity, unit_price_cents \
         from order_items where order_id = $1 order by product_id",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("could not read the order: {e}")))?;

    Ok(Some(OrderView {
        id,
        lines,
        total_cents,
    }))
}

/// How many of each kind the history screen shows. Small enough that it never
/// needs paging, large enough to be a real history.
pub const HISTORY_LIMIT: i64 = 20;

/// One placed order, as the history screen reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderSummary {
    pub id: i64,
    /// Pre-formatted by Postgres, because no timestamp is ever selected into
    /// Rust here — see the module note on dependencies.
    pub placed_on: String,
    pub total_cents: i32,
    pub lines: Vec<Line>,
}

impl OrderSummary {
    pub fn total_label(&self) -> String {
        money(self.total_cents)
    }
}

/// A checkout that ran out of time. Kept so a shopper can see what they lost.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpiredReservation {
    pub id: i64,
    pub expired_on: String,
    pub lines: Vec<Line>,
}

impl ExpiredReservation {
    pub fn total_cents(&self) -> i32 {
        total_cents(&self.lines)
    }

    pub fn total_label(&self) -> String {
        money(self.total_cents())
    }
}

/// Everything the history screen shows, in one round trip.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    pub orders: Vec<OrderSummary>,
    pub expired: Vec<ExpiredReservation>,
}

impl History {
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty() && self.expired.is_empty()
    }
}

/// What the signed-in shopper has bought, and what they let lapse.
///
/// Signed out reads as an empty history rather than an error, matching
/// [`current_reservation`] and `list_cart`.
#[server(endpoint = "order_history")]
pub async fn order_history() -> Result<History, ServerFnError> {
    use self::ssr::*;
    use crate::auth::current_user_from_request;

    let Some(user) = current_user_from_request().await else {
        return Ok(History::default());
    };
    let pool = expect_context::<sqlx::PgPool>();

    // Anything that lapsed while they were away should show as lapsed.
    sweep_expired(&pool).await?;

    let order_rows: Vec<(i64, String, i32)> = sqlx::query_as(
        // Rendered in UTC and labelled as such: the app has no notion of a
        // shopper's timezone, and an unlabelled time would silently be the
        // server's.
        "select id, \
                to_char(placed_at at time zone 'UTC', 'DD Mon YYYY, HH24:MI') || ' UTC', \
                total_cents \
         from orders where user_id = $1 order by placed_at desc limit $2",
    )
    .bind(user.id)
    .bind(HISTORY_LIMIT)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("could not read your orders: {e}")))?;

    let mut orders = Vec::with_capacity(order_rows.len());
    for (id, placed_on, total_cents) in order_rows {
        let lines = sqlx::query_as::<_, Line>(
            "select product_id, title, quantity, unit_price_cents \
             from order_items where order_id = $1 order by product_id",
        )
        .bind(id)
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("could not read your orders: {e}")))?;

        orders.push(OrderSummary {
            id,
            placed_on,
            total_cents,
            lines,
        });
    }

    let expired_rows: Vec<(i64, String)> = sqlx::query_as(
        "select id, \
                to_char(coalesce(settled_at, expires_at) at time zone 'UTC', \
                        'DD Mon YYYY, HH24:MI') || ' UTC' \
         from reservations where user_id = $1 and status = 'expired' \
         order by coalesce(settled_at, expires_at) desc limit $2",
    )
    .bind(user.id)
    .bind(HISTORY_LIMIT)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("could not read your checkouts: {e}")))?;

    let mut expired = Vec::with_capacity(expired_rows.len());
    for (id, expired_on) in expired_rows {
        expired.push(ExpiredReservation {
            id,
            expired_on,
            lines: reservation_lines(&pool, id).await?,
        });
    }

    Ok(History { orders, expired })
}

// ── reactive context ───────────────────────────────────────────────────────

/// The pending checkout, plus the two actions that move it along.
#[derive(Clone, Copy)]
pub struct Checkout {
    reservation: Resource<Result<Option<ReservationView>, ServerFnError>>,
    pub start: ServerAction<StartCheckout>,
    pub pay: ServerAction<Pay>,
}

impl Checkout {
    pub fn load(auth: Auth) -> Self {
        let start = ServerAction::<StartCheckout>::new();
        let pay = ServerAction::<Pay>::new();

        let version = move || (auth.version(), start.version().get(), pay.version().get());
        let reservation = Resource::new_blocking(version, |_| current_reservation());

        Self {
            reservation,
            start,
            pay,
        }
    }

    pub fn from_context() -> Self {
        expect_context()
    }

    /// Bumped when a checkout is opened or paid for. [`crate::cart::Cart`]
    /// tracks this: opening a checkout empties the bag server-side, and nothing
    /// else would tell the cart resource to re-read.
    pub fn version(&self) -> usize {
        self.start.version().get() + self.pay.version().get()
    }

    /// A failed read reads as nothing pending, so the bag screen survives it.
    pub fn with<T>(&self, f: impl FnOnce(Option<&ReservationView>) -> T) -> T {
        self.reservation.with(|loaded| match loaded {
            Some(Ok(Some(view))) => f(Some(view)),
            _ => f(None),
        })
    }

    pub fn pending_reservation(&self) -> Option<ReservationView> {
        self.with(|view| view.cloned())
    }

    pub fn has_pending(&self) -> bool {
        self.with(|view| view.is_some())
    }

    pub fn busy(&self) -> bool {
        self.start.pending().get() || self.pay.pending().get()
    }

    pub fn last_error(&self) -> Option<String> {
        fn failed(value: Option<Result<i64, ServerFnError>>) -> Option<String> {
            match value {
                Some(Err(e)) => Some(e.to_string()),
                _ => None,
            }
        }
        failed(self.start.value().get()).or_else(|| failed(self.pay.value().get()))
    }

    pub fn begin(&self) {
        self.start.dispatch(StartCheckout {});
    }

    pub fn pay_for(&self, reservation_id: i64) {
        self.pay.dispatch(Pay { reservation_id });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(product_id: i32, quantity: i32, unit_price_cents: i32) -> Line {
        Line {
            product_id,
            title: format!("Product {product_id}"),
            quantity,
            unit_price_cents,
        }
    }

    #[test]
    fn a_line_costs_its_price_times_its_quantity() {
        assert_eq!(line(1, 3, 999).subtotal_cents(), 2997);
        assert_eq!(line(1, 1, 0).subtotal_cents(), 0);
    }

    #[test]
    fn the_total_is_every_line() {
        let lines = [line(1, 3, 999), line(2, 1, 12999)];
        assert_eq!(total_cents(&lines), 2997 + 12999);
        assert_eq!(total_cents(&[]), 0);
    }

    #[test]
    fn the_countdown_reads_in_whole_minutes() {
        assert_eq!(expires_label(15 * 60), "expires in 15 minutes");
        assert_eq!(expires_label(14 * 60 + 59), "expires in 14 minutes");
        assert_eq!(expires_label(120), "expires in 2 minutes");
        assert_eq!(expires_label(119), "expires in 1 minute");
        assert_eq!(expires_label(59), "expires in under a minute");
        assert_eq!(expires_label(0), "expired");
        // The query clamps at zero, but the label must not go strange if it did not.
        assert_eq!(expires_label(-5), "expired");
    }
}
