-- The product catalogue. Everything else in the app (bag, saved list, filters)
-- lives in the browser; this table is the one piece of server state.
create table if not exists products (
    id            integer      primary key,
    slug          text         not null unique,
    title         text         not null,
    category      text         not null,
    description   text         not null,
    -- Editorial: the buying desk's note, shown on the product page when set.
    note          text,

    -- Money as integer cents; never a float.
    price_cents   integer      not null check (price_cents >= 0),
    discount_pct  real         not null default 0,
    rating        real,

    stock         integer      not null default 0,
    availability  text         not null,

    brand         text,
    sku           text,
    weight_grams  integer,
    width_mm      real,
    height_mm     real,
    depth_mm      real,
    warranty      text,
    shipping      text,
    return_policy text,
    min_order     integer,

    thumbnail_url text,
    tags          text[]       not null default '{}',

    created_at    timestamptz  not null default now(),
    updated_at    timestamptz  not null default now()
);

create index if not exists products_category_idx on products (category);
create index if not exists products_price_idx on products (price_cents);
