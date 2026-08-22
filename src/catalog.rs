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
    /// The index number the design prints beside each entry.
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

    /// The one-liner under the title in the index: the description's first
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
        if i > 0 && (whole.len() - i) % 3 == 0 {
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
        UNDER_400 => p.price_cents < 400_00,
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
