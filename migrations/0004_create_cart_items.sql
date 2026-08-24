-- A signed-in shopper's bag. One row per (user, product); a guest has no bag at
-- all, so there is nothing to merge at sign-in.
--
-- There is no inventory yet: nothing here reserves or deducts `products.stock`.
-- The only ceiling is a flat per-line cap, mirrored by `cart::MAX_QUANTITY`.
create table cart_items (
    user_id    bigint      not null references users (id)    on delete cascade,
    product_id integer     not null references products (id) on delete cascade,
    quantity   integer     not null check (quantity > 0 and quantity <= 99),

    -- The bag lists in the order things were added, which is what the in-memory
    -- `Vec` used to give for free.
    added_at   timestamptz not null default now(),
    updated_at timestamptz not null default now(),

    primary key (user_id, product_id)
);

create index cart_items_user_idx on cart_items (user_id, added_at);
