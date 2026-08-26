-- Physical inventory, one row per product.
--
-- `on_hand` is what is in the building. `reserved` is what is already spoken
-- for — taken by an order that has not shipped — and `available` is the
-- difference: what a shopper can still buy.
--
-- Reservation is a later step. Nothing writes `reserved` yet, so it stays 0 and
-- `available` equals `on_hand`. The bag does not reserve: `cart_items` is a
-- shopping list, not a claim on stock.
--
-- This table is the stock record. `products.stock` and `products.availability`
-- come from the upstream importer and are deliberately left alone — the screens
-- still read `availability` for the "In stock" chip, so the two can disagree
-- until a later step moves the display onto these numbers.
create table inventory (
    product_id integer     primary key references products (id) on delete cascade,

    on_hand    integer     not null default 0 check (on_hand >= 0),
    reserved   integer     not null default 0 check (reserved >= 0),

    -- Derived rather than stored twice, so it cannot drift out of step with the
    -- two columns it comes from, and so a query can filter on it directly.
    available  integer     generated always as (on_hand - reserved) stored,

    updated_at timestamptz not null default now(),

    -- Never promise more than is in the building. This is what keeps
    -- `available` from going negative.
    constraint inventory_reserved_within_on_hand check (reserved <= on_hand)
);

-- Every product in the catalogue at this point starts with ten on the shelf.
--
-- Products imported *after* this migration get no row, and a missing row is not
-- the same as zero stock — whatever reads this table next has to decide which
-- it means, or `import_products` has to start creating rows.
insert into inventory (product_id, on_hand)
select id, 10 from products;
