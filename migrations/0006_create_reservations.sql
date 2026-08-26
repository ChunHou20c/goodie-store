-- Checkout: a claim on stock, and the order it becomes.
--
-- Pressing Checkout creates a `pending` reservation and raises
-- `inventory.reserved`, which drops `available` — the goods are spoken for. The
-- reservation then ends one of two ways: paid, which takes the goods out of the
-- building (`on_hand` and `reserved` both fall), or expired, which puts them
-- back on the shelf (`reserved` falls alone).
create type reservation_status as enum ('pending', 'expired', 'fulfilled');

create table reservations (
    id         bigint      generated always as identity primary key,
    user_id    bigint      not null references users (id) on delete cascade,
    status     reservation_status not null default 'pending',

    created_at timestamptz not null default now(),
    expires_at timestamptz not null,
    -- When it stopped being pending, either way.
    settled_at timestamptz,

    constraint reservations_settled_when_not_pending
        check ((status = 'pending') = (settled_at is null))
);

-- One pending reservation per shopper, enforced here rather than by remembering
-- to check: pressing Checkout again cannot open a second claim on stock, it can
-- only take you back to the one you have.
create unique index reservations_one_pending_per_user
    on reservations (user_id) where status = 'pending';

create table reservation_items (
    reservation_id   bigint  not null references reservations (id) on delete cascade,
    product_id       integer not null references products (id),
    quantity         integer not null check (quantity > 0),
    -- The price the shopper was shown, not whatever it is when they pay.
    unit_price_cents integer not null check (unit_price_cents >= 0),

    primary key (reservation_id, product_id)
);

create table orders (
    id             bigint  generated always as identity primary key,
    user_id        bigint  not null references users (id) on delete cascade,
    -- One order per reservation. This is what makes paying twice impossible
    -- rather than merely unlikely.
    reservation_id bigint  not null unique references reservations (id),
    total_cents    integer not null check (total_cents >= 0),
    placed_at      timestamptz not null default now()
);

create table order_items (
    order_id         bigint  not null references orders (id) on delete cascade,
    -- Deliberately no foreign key, and the title copied rather than joined: an
    -- order is a historical record and has to keep reading correctly even if the
    -- catalogue is re-imported or a product is renamed underneath it.
    product_id       integer not null,
    title            text    not null,
    quantity         integer not null check (quantity > 0),
    unit_price_cents integer not null check (unit_price_cents >= 0),

    primary key (order_id, product_id)
);

create index orders_user_idx on orders (user_id, placed_at desc);
