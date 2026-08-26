//! The product catalogue — the one part of the app that lives on the server.
//!
//! Rows come from Postgres via [`list_products`]; everything the screens do
//! with them afterwards (searching, filtering, sorting) happens in the browser
//! over the loaded list.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// A row of `products`, as the screens want to read it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Product {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub category: String,
    pub description: String,
    /// The buying desk's note. Nothing in the seed writes one yet.
    pub note: Option<String>,
    pub price_cents: i32,
    pub rating: Option<f32>,
    pub stock: i32,
    pub availability: String,
    pub brand: Option<String>,
    pub sku: Option<String>,
    pub weight_grams: Option<i32>,
    pub width_mm: Option<f32>,
    pub height_mm: Option<f32>,
    pub depth_mm: Option<f32>,
    pub warranty: Option<String>,
    pub shipping: Option<String>,
    pub return_policy: Option<String>,
    pub min_order: Option<i32>,
    pub thumbnail_url: Option<String>,
    pub tags: Vec<String>,
}

/// The chips that are not categories.
pub const UNDER_400: &str = "Under $400";
pub const IN_STOCK: &str = "In stock";

impl Product {
    /// The catalogue number printed beside each entry.
    pub fn num(&self) -> String {
        format!("{:02}", self.id)
    }

    pub fn price_label(&self) -> String {
        money(self.price_cents)
    }

    pub fn in_stock(&self) -> bool {
        self.availability == "In Stock"
    }

    /// `mobile-accessories` → `Mobile accessories`.
    pub fn category_label(&self) -> String {
        category_label(&self.category)
    }

    /// The one-liner under the title on the shelf: the description's first
    /// sentence, which is how these read.
    pub fn line(&self) -> String {
        let text = self.description.trim();
        match text.find(". ") {
            Some(end) => text[..=end].to_string(),
            None => text.to_string(),
        }
    }

    /// The spec table, from whichever columns this row actually has.
    pub fn specs(&self) -> Vec<(&'static str, String)> {
        let mut specs = Vec::new();
        if let Some(brand) = &self.brand {
            specs.push(("Brand", brand.clone()));
        }
        if let Some(sku) = &self.sku {
            specs.push(("SKU", sku.clone()));
        }
        if let Some(rating) = self.rating {
            specs.push(("Rating", format!("{rating:.2} / 5")));
        }
        if let (Some(w), Some(h), Some(d)) = (self.width_mm, self.height_mm, self.depth_mm) {
            specs.push(("Dimensions", format!("{w:.1} × {h:.1} × {d:.1} mm")));
        }
        if let Some(weight) = self.weight_grams {
            specs.push(("Weight", format!("{weight} g")));
        }
        if let Some(warranty) = &self.warranty {
            specs.push(("Warranty", warranty.clone()));
        }
        if let Some(shipping) = &self.shipping {
            specs.push(("Shipping", shipping.clone()));
        }
        if let Some(policy) = &self.return_policy {
            specs.push(("Returns", policy.clone()));
        }
        specs
    }

    fn haystack(&self) -> String {
        format!("{} {} {}", self.title, self.category, self.description).to_lowercase()
    }
}

pub fn category_label(category: &str) -> String {
    let spaced = category.replace('-', " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => spaced,
    }
}

/// `36999_99` → `"$36,999.99"`.
pub fn money(cents: i32) -> String {
    let negative = cents < 0;
    let cents = cents.unsigned_abs();
    let whole = (cents / 100).to_string();
    let mut out = String::with_capacity(whole.len() + 6);
    if negative {
        out.push('-');
    }
    out.push('$');
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out.push_str(&format!(".{:02}", cents % 100));
    out
}

/// The chip row: every category the catalogue actually has, most stocked
/// first, then price and availability.
pub fn chips(products: &[Product]) -> Vec<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for p in products {
        match counts.iter_mut().find(|(c, _)| *c == p.category) {
            Some(entry) => entry.1 += 1,
            None => counts.push((p.category.clone(), 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut chips: Vec<String> = counts.into_iter().map(|(c, _)| c).collect();
    chips.push(UNDER_400.to_string());
    chips.push(IN_STOCK.to_string());
    chips
}

/// One chip's predicate. The two trailing chips read a different field each;
/// anything else is a category.
fn chip_matches(chip: &str, p: &Product) -> bool {
    match chip {
        UNDER_400 => p.price_cents < 40_000, // $400.00 in cents
        IN_STOCK => p.in_stock(),
        category => p.category == category,
    }
}

/// Free text over title/category/description, then every active chip. Chips
/// are ANDed, as in the prototype — two categories at once find nothing, and
/// the empty state offers a way back out.
pub fn search<'a>(products: &'a [Product], query: &str, filters: &[String]) -> Vec<&'a Product> {
    let q = query.trim().to_lowercase();
    products
        .iter()
        .filter(|p| q.is_empty() || p.haystack().contains(&q))
        .filter(|p| filters.iter().all(|f| chip_matches(f, p)))
        .collect()
}

/// Every product, in catalogue order.
#[server(endpoint = "list_products")]
pub async fn list_products() -> Result<Vec<Product>, ServerFnError> {
    let pool = expect_context::<sqlx::PgPool>();
    sqlx::query_as::<_, Product>(
        "select id, slug, title, category, description, note, price_cents, rating, stock, \
         availability, brand, sku, weight_grams, width_mm, height_mm, depth_mm, warranty, \
         shipping, return_policy, min_order, thumbnail_url, tags \
         from products order by id",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("catalog query failed: {e}")))
}

/// The loaded catalogue, provided in context by `App`.
///
/// One blocking resource for the whole app: it resolves on the server, is
/// serialized into the response, and is read straight out of that on the
/// client — so screens read it synchronously and client-side navigation never
/// refetches it.
#[derive(Clone, Copy)]
pub struct Catalog {
    resource: Resource<Result<Vec<Product>, ServerFnError>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Loading,
    Ready,
    Failed(String),
}

impl Catalog {
    pub fn load() -> Self {
        Self {
            resource: Resource::new_blocking(|| (), |_| list_products()),
        }
    }

    pub fn from_context() -> Self {
        expect_context()
    }

    pub fn status(&self) -> Status {
        self.resource.with(|loaded| match loaded {
            None => Status::Loading,
            Some(Ok(_)) => Status::Ready,
            Some(Err(e)) => Status::Failed(e.to_string()),
        })
    }

    /// Read the loaded rows without cloning them. Empty until the resource
    /// resolves, which is why every caller sits inside a `WithCatalog`.
    pub fn with<T>(&self, f: impl FnOnce(&[Product]) -> T) -> T {
        self.resource.with(|loaded| match loaded {
            Some(Ok(products)) => f(products),
            _ => f(&[]),
        })
    }

    pub fn len(&self) -> usize {
        self.with(<[Product]>::len)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn by_slug(&self, slug: &str) -> Option<Product> {
        self.with(|products| products.iter().find(|p| p.slug == slug).cloned())
    }

    pub fn take(&self, n: usize) -> Vec<Product> {
        self.with(|products| products.iter().take(n).cloned().collect())
    }

    /// Re-read the catalogue — the admin console calls this after an import.
    pub fn refetch(&self) {
        self.resource.refetch();
    }
}

/// Stock an import puts on the shelf, per product fetched.
///
/// This is a **delivery**, not a correction: every run adds this much to every
/// product in the range, so importing the same range twice leaves twice the
/// stock. That is deliberate — re-running an import is how you record another
/// shipment arriving — but it does mean the catalogue half of an import is
/// idempotent while the stock half is not.
pub const IMPORT_STOCK_UNITS: i32 = 10;

/// What one run of [`import_products`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    pub fetched: usize,
    pub inserted: usize,
    pub updated: usize,
    /// Units added to `inventory` by this run — [`IMPORT_STOCK_UNITS`] per
    /// product fetched, new or not.
    pub stocked_units: usize,
    /// `total` as reported by the upstream API.
    pub total_available: usize,
}

impl ImportReport {
    pub fn summary(&self) -> String {
        format!(
            "{} imported, {} refreshed, +{} in stock — {} available upstream",
            self.inserted, self.updated, self.stocked_units, self.total_available
        )
    }
}

/// Pull a slice of the upstream catalogue into `products`, and put stock on the
/// shelf for everything it pulled. **Admin only.**
///
/// The browser never talks to dummyjson: the server fetches, maps and upserts,
/// and only for an admin. Re-running the same range leaves the catalogue
/// unchanged — rows are matched on the upstream id and refreshed, and `note` is
/// left alone because it is ours, not theirs.
///
/// The stock half is **not** idempotent: every run adds [`IMPORT_STOCK_UNITS`]
/// to every product it fetched, new or not, because an import is a delivery
/// arriving rather than a correction. Importing the same range twice therefore
/// leaves twice the stock. Both halves share one transaction, so a run that
/// fails partway leaves neither.
#[server(endpoint = "import_products")]
pub async fn import_products(limit: u32, skip: u32) -> Result<ImportReport, ServerFnError> {
    use crate::auth::require_admin;

    require_admin().await?;

    let pool = expect_context::<sqlx::PgPool>();
    let limit = limit.clamp(1, 100);
    let payload = self::import::fetch(limit, skip).await?;

    let mut report = ImportReport {
        fetched: payload.products.len(),
        total_available: payload.total,
        ..Default::default()
    };

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("import failed to start: {e}")))?;

    let mut claimed: Vec<String> = Vec::with_capacity(payload.products.len());
    for api in &payload.products {
        let row = self::import::to_row(api);
        let slug = self::import::free_slug(&mut tx, &row, &claimed).await?;
        claimed.push(slug.clone());

        if self::import::upsert(&mut tx, &row, &slug).await? {
            report.inserted += 1;
        } else {
            report.updated += 1;
        }

        // Same transaction as the upsert: a run that fails partway leaves
        // neither the row nor the stock behind.
        self::import::add_stock(&mut tx, row.id, IMPORT_STOCK_UNITS).await?;
        report.stocked_units += IMPORT_STOCK_UNITS as usize;
    }

    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("import failed to commit: {e}")))?;

    Ok(report)
}

/// Fetching and mapping the upstream payload.
///
/// This mapping is the canonical one. `scripts/generate-seed-sql.py` expresses
/// the same rules in Python purely to regenerate the committed offline seed;
/// `mapping_matches_the_committed_seed` below is the guard that they agree.
#[cfg(feature = "ssr")]
mod import {
    use super::*;
    use serde::Deserialize;
    use sqlx::{Postgres, Transaction};
    use std::time::Duration;

    /// Only the fields we store.
    const SELECT: &str = "id,title,description,category,price,discountPercentage,rating,stock,\
brand,sku,weight,dimensions,warrantyInformation,shippingInformation,availabilityStatus,\
returnPolicy,minimumOrderQuantity,tags,thumbnail";

    #[derive(Debug, Deserialize)]
    pub struct ApiPayload {
        pub products: Vec<ApiProduct>,
        pub total: usize,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ApiProduct {
        pub id: i32,
        pub title: String,
        #[serde(default)]
        pub description: Option<String>,
        pub category: String,
        pub price: f64,
        #[serde(default)]
        pub discount_percentage: Option<f32>,
        #[serde(default)]
        pub rating: Option<f32>,
        #[serde(default)]
        pub stock: Option<i32>,
        #[serde(default)]
        pub brand: Option<String>,
        #[serde(default)]
        pub sku: Option<String>,
        #[serde(default)]
        pub weight: Option<f32>,
        #[serde(default)]
        pub dimensions: Option<ApiDimensions>,
        #[serde(default)]
        pub warranty_information: Option<String>,
        #[serde(default)]
        pub shipping_information: Option<String>,
        #[serde(default)]
        pub availability_status: Option<String>,
        #[serde(default)]
        pub return_policy: Option<String>,
        #[serde(default)]
        pub minimum_order_quantity: Option<i32>,
        #[serde(default)]
        pub tags: Vec<String>,
        #[serde(default)]
        pub thumbnail: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct ApiDimensions {
        pub width: Option<f32>,
        pub height: Option<f32>,
        pub depth: Option<f32>,
    }

    /// A `products` row, ready to bind.
    #[derive(Debug, PartialEq)]
    pub struct Row {
        pub id: i32,
        pub slug: String,
        pub title: String,
        pub category: String,
        pub description: String,
        pub price_cents: i32,
        pub discount_pct: f32,
        pub rating: Option<f32>,
        pub stock: i32,
        pub availability: String,
        pub brand: Option<String>,
        pub sku: Option<String>,
        pub weight_grams: Option<i32>,
        pub width_mm: Option<f32>,
        pub height_mm: Option<f32>,
        pub depth_mm: Option<f32>,
        pub warranty: Option<String>,
        pub shipping: Option<String>,
        pub return_policy: Option<String>,
        pub min_order: Option<i32>,
        pub thumbnail_url: Option<String>,
        pub tags: Vec<String>,
    }

    pub fn slugify(title: &str) -> String {
        let mut slug = String::with_capacity(title.len());
        for c in title.to_lowercase().chars() {
            if c.is_ascii_alphanumeric() {
                slug.push(c);
            } else if !slug.ends_with('-') {
                slug.push('-');
            }
        }
        let slug = slug.trim_matches('-').to_string();
        if slug.is_empty() {
            "product".to_string()
        } else {
            slug
        }
    }

    pub fn to_row(api: &ApiProduct) -> Row {
        let dims = api.dimensions.as_ref();
        Row {
            id: api.id,
            slug: slugify(&api.title),
            title: api.title.clone(),
            category: api.category.clone(),
            description: api.description.clone().unwrap_or_default(),
            price_cents: (api.price * 100.0).round() as i32,
            discount_pct: api.discount_percentage.unwrap_or(0.0),
            rating: api.rating,
            stock: api.stock.unwrap_or(0),
            availability: api
                .availability_status
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
            brand: api.brand.clone(),
            sku: api.sku.clone(),
            weight_grams: api.weight.map(|w| w.round() as i32),
            width_mm: dims.and_then(|d| d.width),
            height_mm: dims.and_then(|d| d.height),
            depth_mm: dims.and_then(|d| d.depth),
            warranty: api.warranty_information.clone(),
            shipping: api.shipping_information.clone(),
            return_policy: api.return_policy.clone(),
            min_order: api.minimum_order_quantity,
            thumbnail_url: api.thumbnail.clone(),
            tags: api.tags.clone(),
        }
    }

    pub async fn fetch(limit: u32, skip: u32) -> Result<ApiPayload, ServerFnError> {
        let url =
            format!("https://dummyjson.com/products?limit={limit}&skip={skip}&select={SELECT}");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| ServerFnError::new(format!("could not build an http client: {e}")))?;

        client
            .get(url)
            .send()
            .await
            .map_err(|e| ServerFnError::new(format!("upstream request failed: {e}")))?
            .error_for_status()
            .map_err(|e| ServerFnError::new(format!("upstream returned an error: {e}")))?
            .json::<ApiPayload>()
            .await
            .map_err(|e| ServerFnError::new(format!("could not read the upstream payload: {e}")))
    }

    /// `slug` is unique, so resolve collisions before inserting rather than
    /// letting a constraint abort the transaction. A clash with a *different*
    /// product — upstream or earlier in this batch — takes the id as a suffix.
    pub async fn free_slug(
        tx: &mut Transaction<'_, Postgres>,
        row: &Row,
        claimed: &[String],
    ) -> Result<String, ServerFnError> {
        let taken_here = claimed.contains(&row.slug);
        let taken_in_db: Option<(i32,)> =
            sqlx::query_as("select id from products where slug = $1 and id <> $2")
                .bind(&row.slug)
                .bind(row.id)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| ServerFnError::new(format!("slug lookup failed: {e}")))?;

        Ok(if taken_here || taken_in_db.is_some() {
            format!("{}-{}", row.slug, row.id)
        } else {
            row.slug.clone()
        })
    }

    /// Returns true when the row was inserted, false when it was refreshed.
    /// `xmax = 0` is the standard way to tell those apart in an upsert.
    /// Put `units` on the shelf for one product.
    ///
    /// A product the catalogue has not seen before gets its first `inventory`
    /// row; one that is already stocked has the units added to what is there.
    /// `reserved` is never touched — a delivery cannot disturb what is already
    /// spoken for, and since this only raises `on_hand` it cannot trip the
    /// `reserved <= on_hand` constraint.
    pub async fn add_stock(
        tx: &mut Transaction<'_, Postgres>,
        product_id: i32,
        units: i32,
    ) -> Result<(), ServerFnError> {
        sqlx::query(
            "insert into inventory (product_id, on_hand) values ($1, $2) \
             on conflict (product_id) do update \
             set on_hand = inventory.on_hand + excluded.on_hand, updated_at = now()",
        )
        .bind(product_id)
        .bind(units)
        .execute(&mut **tx)
        .await
        .map_err(|e| ServerFnError::new(format!("could not stock product {product_id}: {e}")))?;

        Ok(())
    }

    pub async fn upsert(
        tx: &mut Transaction<'_, Postgres>,
        row: &Row,
        slug: &str,
    ) -> Result<bool, ServerFnError> {
        let (inserted,): (bool,) = sqlx::query_as(
            "insert into products (id, slug, title, category, description, price_cents, \
             discount_pct, rating, stock, availability, brand, sku, weight_grams, width_mm, \
             height_mm, depth_mm, warranty, shipping, return_policy, min_order, thumbnail_url, tags) \
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
             $18, $19, $20, $21, $22) \
             on conflict (id) do update set \
             slug = excluded.slug, title = excluded.title, category = excluded.category, \
             description = excluded.description, price_cents = excluded.price_cents, \
             discount_pct = excluded.discount_pct, rating = excluded.rating, \
             stock = excluded.stock, availability = excluded.availability, \
             brand = excluded.brand, sku = excluded.sku, weight_grams = excluded.weight_grams, \
             width_mm = excluded.width_mm, height_mm = excluded.height_mm, \
             depth_mm = excluded.depth_mm, warranty = excluded.warranty, \
             shipping = excluded.shipping, return_policy = excluded.return_policy, \
             min_order = excluded.min_order, thumbnail_url = excluded.thumbnail_url, \
             tags = excluded.tags, updated_at = now() \
             returning (xmax = 0) as inserted",
        )
        .bind(row.id)
        .bind(slug)
        .bind(&row.title)
        .bind(&row.category)
        .bind(&row.description)
        .bind(row.price_cents)
        .bind(row.discount_pct)
        .bind(row.rating)
        .bind(row.stock)
        .bind(&row.availability)
        .bind(&row.brand)
        .bind(&row.sku)
        .bind(row.weight_grams)
        .bind(row.width_mm)
        .bind(row.height_mm)
        .bind(row.depth_mm)
        .bind(&row.warranty)
        .bind(&row.shipping)
        .bind(&row.return_policy)
        .bind(row.min_order)
        .bind(&row.thumbnail_url)
        .bind(&row.tags)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| ServerFnError::new(format!("upsert failed for id {}: {e}", row.id)))?;

        Ok(inserted)
    }
}

/// Guards the one duplicated rule in the project: the upstream→row mapping is
/// written in Rust here and in Python in `scripts/generate-seed-sql.py`. These
/// tests replay the committed payload through the Rust path and check it against
/// what the Python generator actually wrote into `0002_seed_products.sql`.
#[cfg(all(test, feature = "ssr"))]
mod import_tests {
    use super::import::{slugify, to_row, ApiPayload};

    fn seed_payload() -> ApiPayload {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/seed/dummyjson-products.json");
        let json = std::fs::read_to_string(path).expect("committed seed payload");
        serde_json::from_str(&json).expect("payload parses into the importer's shape")
    }

    /// `(id, slug)` pairs as the generator wrote them.
    fn seeded_slugs() -> Vec<(i32, String)> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/migrations/0002_seed_products.sql"
        );
        let sql = std::fs::read_to_string(path).expect("generated seed sql");
        sql.lines()
            .filter_map(|line| {
                let line = line.trim().strip_prefix('(')?;
                let (id, rest) = line.split_once(", '")?;
                let (slug, _) = rest.split_once('\'')?;
                Some((id.parse().ok()?, slug.to_string()))
            })
            .collect()
    }

    #[test]
    fn slugs_match_the_generated_seed() {
        let payload = seed_payload();
        let seeded = seeded_slugs();
        // The seed is a prefix of the payload — the rest arrives through the
        // admin import — so compare pairwise and let `zip` stop at the shorter.
        assert!(!seeded.is_empty(), "parsed the seeded rows");
        assert!(seeded.len() <= payload.products.len());

        // Same rule `free_slug` applies within a batch: the base slug, or the
        // base plus the product id when something else already claimed it.
        let mut claimed: Vec<String> = Vec::new();
        for (api, (id, slug)) in payload.products.iter().zip(seeded) {
            assert_eq!(api.id, id, "seed order follows the payload");

            let base = slugify(&api.title);
            let expected = if claimed.contains(&base) {
                format!("{base}-{}", api.id)
            } else {
                base
            };
            claimed.push(expected.clone());
            assert_eq!(expected, slug, "slug rule agrees for id {id}");
        }
    }

    #[test]
    fn maps_money_and_units_like_the_generator() {
        let payload = seed_payload();
        let first = to_row(&payload.products[0]);

        assert_eq!(first.id, 1);
        assert_eq!(first.slug, "essence-mascara-lash-princess");
        assert_eq!(first.price_cents, 999, "$9.99 stored as cents");
        assert_eq!(first.availability, "In Stock");
        assert_eq!(first.weight_grams, Some(4));
        assert_eq!(first.sku.as_deref(), Some("BEA-ESS-ESS-001"));
        assert_eq!(first.tags, vec!["beauty", "mascara"]);
        assert!(first.thumbnail_url.is_some());

        // Every row must satisfy the table's constraints.
        for api in &payload.products {
            let row = to_row(api);
            assert!(
                row.price_cents >= 0,
                "price_cents check constraint, id {}",
                row.id
            );
            assert!(!row.slug.is_empty());
            assert!(!row.availability.is_empty());
        }
    }

    #[test]
    fn slugify_handles_awkward_titles() {
        assert_eq!(slugify("Calvin Klein CK One"), "calvin-klein-ck-one");
        assert_eq!(slugify("  Spaced   Out  "), "spaced-out");
        assert_eq!(slugify("Symbols !@#$ Only"), "symbols-only");
        assert_eq!(slugify("!!!"), "product");
        assert_eq!(slugify(""), "product");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product(id: i32, category: &str, price_cents: i32, availability: &str) -> Product {
        Product {
            id,
            slug: format!("p{id}"),
            title: format!("Product {id}"),
            category: category.to_string(),
            description: "First sentence. Second sentence.".to_string(),
            note: None,
            price_cents,
            rating: None,
            stock: 1,
            availability: availability.to_string(),
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

    #[test]
    fn money_groups_thousands_and_keeps_cents() {
        assert_eq!(money(999), "$9.99");
        assert_eq!(money(18000), "$180.00");
        assert_eq!(money(3699999), "$36,999.99");
        assert_eq!(money(0), "$0.00");
    }

    #[test]
    fn line_is_the_first_sentence() {
        assert_eq!(
            product(1, "beauty", 999, "In Stock").line(),
            "First sentence."
        );
    }

    #[test]
    fn chips_are_categories_by_count_then_price_and_stock() {
        let products = vec![
            product(1, "beauty", 999, "In Stock"),
            product(2, "laptops", 99999, "In Stock"),
            product(3, "beauty", 1999, "Low Stock"),
        ];
        assert_eq!(
            chips(&products),
            vec!["beauty", "laptops", UNDER_400, IN_STOCK]
        );
    }

    #[test]
    fn filters_and_query_compose() {
        let products = vec![
            product(1, "beauty", 999, "In Stock"),
            product(2, "laptops", 99999, "In Stock"),
            product(3, "beauty", 1999, "Low Stock"),
        ];
        let all: Vec<i32> = search(&products, "", &[]).iter().map(|p| p.id).collect();
        assert_eq!(all, vec![1, 2, 3]);

        let cheap_in_stock: Vec<i32> = search(
            &products,
            "",
            &[UNDER_400.to_string(), IN_STOCK.to_string()],
        )
        .iter()
        .map(|p| p.id)
        .collect();
        assert_eq!(cheap_in_stock, vec![1]);

        assert!(search(&products, "product 2", &["beauty".to_string()]).is_empty());
    }
}
