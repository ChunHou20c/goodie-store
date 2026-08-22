# Kessel — `goodie-never-deliver`

A full-stack shop: [Leptos](https://github.com/leptos-rs/leptos) (SSR +
hydration) on [Axum](https://github.com/tokio-rs/axum), styled with Tailwind CSS
v4, with the product catalogue in Postgres. The four screens implement the
*Kessel Shop* design on its Modernist design system — see
[The app](#the-app--kessel-issue-14).

## Quick start

Everything the project needs is pinned in `flake.nix`; there is nothing to
`cargo install`, `npm install -g`, or `rustup add`.

```bash
nix develop           # or: direnv allow
pg-start              # initdb on first run, start Postgres, create `goodie`
cargo leptos watch    # migrate, seed, build and serve on http://127.0.0.1:3000
```

That is the whole setup on a clean checkout. `cargo leptos watch` rebuilds the
server, the WASM bundle and the CSS on save.

## Development environment (Nix)

`nix develop` (or `direnv allow` — an `.envrc` with `use flake` is included) puts
every pinned tool on `PATH`:

| Tool | Source | Notes |
| --- | --- | --- |
| rustc / cargo / clippy / rustfmt / rust-analyzer | `rust-overlay` via `rust-toolchain.toml` | nightly, with the `wasm32-unknown-unknown` target |
| `cargo-leptos` | nixpkgs | built with `no_downloads`, so it uses the tools below from `PATH` |
| `wasm-bindgen` | nixpkgs (`wasm-bindgen-cli_0_2_126`) | must match the `wasm-bindgen` crate exactly |
| `wasm-opt`, `sass`, `tailwindcss` | binaryen, dart-sass, tailwindcss v4 | the other binaries `cargo-leptos` shells out to |
| `node`, `npx` | nodejs 24 | Tailwind plugins, Playwright e2e |
| `postgres`, `psql`, `sqlx` | postgresql 18, sqlx-cli | plus the `pg-*` helpers below |

### Postgres

A throwaway cluster lives in `./.pg` (gitignored) on port 5433, socket in the
checkout, trust auth — nothing touches a system-wide Postgres.

```bash
pg-start    # initdb on first run, start, create the `goodie` database
pg-stop
pg-reset    # delete the cluster entirely
```

`DATABASE_URL` is exported in the shell and points at that cluster; the app
migrates and seeds it on startup (see [The database](#the-database)).

### Bumping wasm-bindgen

The CLI and the crate share a schema version and refuse to work across a
mismatch, so they move together:

1. pick a `wasm-bindgen-cli_0_2_*` attribute in `flake.nix`
2. set the same version in `Cargo.toml` (`wasm-bindgen = { version = "=0.2.x" }`)
   and run `cargo update`

The dev shell prints a warning if `Cargo.lock` and the CLI drift apart.

## The app — Kessel, Issue 14

The four screens implement `Kessel Shop.dc.html` from the *Mobile shopping app
design* Claude Design project, on its **Modernist** design system.

| Route | Screen |
| --- | --- |
| `/` | The issue: cover story, then the numbered index |
| `/search` | Filter chips, cycling sort, two-column results |
| `/p/:id` | Photograph, price, spec table, the desk's note |
| `/bag` | Lines with steppers, totals, checkout |

- `src/catalog.rs` — the `Product` row, the `list_products` server function, the
  chip predicates and the search. The only part of the app that talks to the
  database.
- `src/shop.rs` — bag, saved list, search controls and the toast, in one struct
  provided through context. Holds product ids and nothing else.
- `src/app.rs` — the shell: top bar, sticky action, bottom tabs. All of the
  chrome is derived from the route, not from a screen flag.
- `src/screens/`, `src/ui.rs` — one file per screen; the Lucide icon set, the
  photo slot and the `WithCatalog` suspense gate.

### What is server state and what is not

The catalogue is the one thing that comes from Postgres. The bag, the saved
list, the query, the chips and the sort all live in the tab — they never reach
the server, and reloading the page empties them.

`list_products` is a Leptos server function behind a **blocking resource**, so
it resolves during SSR, is serialized into the HTML, and is read straight out of
that when the client hydrates. Screens then filter and sort the loaded rows in
the browser, and client-side navigation between the four screens costs no
further requests. It is one query per page load and 194 rows on the wire — fine
at this size, and the place to add server-side search and pagination when the
catalogue outgrows it. The same function is also exposed at
`POST /api/list_products` for the hydrated client.

## The database

Postgres holds one table, `products` (see `migrations/0001_create_products.sql`):
money as integer cents, one row per object, a unique `slug` that is also the URL
segment at `/p/:slug`, and a nullable `note` for the buying desk's editorial.

### Running the migrations

There are two ways to apply them, and they are interchangeable — both read the
same `./migrations` directory and the same `_sqlx_migrations` bookkeeping table.

**1. Automatically, when the app starts.** `src/main.rs` runs

```rust
sqlx::migrate!("./migrations").run(&pool).await
```

before it binds the port, so the normal loop needs no migration step at all:

```bash
pg-start              # initdb on first run, start the cluster, create `goodie`
cargo leptos watch    # migrates, seeds, serves on http://127.0.0.1:3000
```

From an empty cluster that is the whole setup: schema and 194 products.

**2. By hand, with `sqlx-cli`** (already in the dev shell). Useful for applying
a migration without restarting the server, or for inspecting state:

```bash
sqlx migrate info     # what is applied and what is pending
sqlx migrate run      # apply everything pending
```

Both read `DATABASE_URL`, which the dev shell exports for you. `sqlx migrate run`
is idempotent — running it against an up-to-date database does nothing.

### Adding a migration

```bash
sqlx migrate add <description>        # writes migrations/<n>_<description>.sql
```

Files are applied in numeric order and the leading number is the version. Write
plain SQL; `create table if not exists` and friends keep re-runs harmless.

The migrations are compiled **into the binary** by `sqlx::migrate!`, so editing
anything under `migrations/` triggers a rebuild of the crate — `cargo leptos
watch` picks that up on its own.

### Two things that will bite

**Never edit a migration that has already been applied.** sqlx stores a checksum
per version and refuses to continue when the file no longer matches:

```
error: migration 2 was previously applied but has been modified
```

`sqlx migrate info` then shows `2/installed (different checksum)`. The fix in
development is to throw the cluster away — `pg-reset && pg-start` — and let it
re-apply from scratch. In anything you cannot reset, add a new migration instead.

**There are no down-migrations.** The files were created without `-r`, so
`sqlx migrate revert` reports "No migrations available to revert". Rolling back
in development means `pg-reset`. If you want reversible migrations from here on,
add them with `sqlx migrate add -r <description>`, which writes a `.up.sql` and a
`.down.sql` pair.

### The seed data

Seed data is **committed**, not fetched at boot:

- `seed/dummyjson-products.json` — 194 products from
  [dummyjson.com/products](https://dummyjson.com/products), verbatim.
- `migrations/0002_seed_products.sql` — the insert statements, generated from
  that file by `scripts/generate-seed-sql.py`. Edit the payload or the
  generator, never the SQL. The inserts end in `on conflict (id) do nothing`, so
  applying them twice is safe.

To refresh it:

```bash
FIELDS=id,title,description,category,price,discountPercentage,rating,stock,brand,\
sku,weight,dimensions,warrantyInformation,shippingInformation,availabilityStatus,\
returnPolicy,minimumOrderQuantity,tags,thumbnail,images
curl -s "https://dummyjson.com/products?limit=0&select=$FIELDS" \
    -o seed/dummyjson-products.json
python3 scripts/generate-seed-sql.py
pg-reset && pg-start          # the seed migration's checksum has changed
cargo leptos watch
```

That last step is the checksum rule above: regenerating `0002` after it has been
applied invalidates it, so the database has to start clean.

The seed brings real categories (24 of them), prices from $0.79 to $36,999.99,
availability and product photography, so the chip row, the empty states and the
price sort all exercise real data.

Three deliberate departures from the prototype:

- The prototype switches screens in local state; here each screen is a **real
  route**, so a product is linkable, the back button works and the four screens
  server-render. Navigation is `<A>`, so it also works before hydration.
- The prototype seeds a bag with one item. This starts **empty** — a shop that
  puts something in your bag on first load would be wrong, and the empty state
  is part of the design anyway.

- The prototype's six hand-written objects are replaced by the seeded
  catalogue, so the chip row is now every category the data has (sorted by how
  many products carry it) followed by "Under $400" and "In stock". Chips are
  still ANDed, as in the prototype — two categories at once find nothing, and
  the empty state offers the way back out.

The toast is anchored just above the bottom chrome rather than the design's flat
96px, which landed it on top of the button it reports on. The product page's
"From Issue 14" panel only appears when a row has a `note`; nothing in the seed
writes one.

## Styling (Tailwind CSS v4)

`style/tailwind.css` is the only stylesheet entrypoint. It carries the
Modernist tokens — colours and their 100–900 ramps, Archivo, zero radius, the
shadow scale — in an `@theme` block, so they come out as ordinary utilities
(`bg-accent`, `text-accent-700`, `border-ink/40`), plus a small `@layer
components` for the system's `.btn`:

```css
@import url('https://fonts.googleapis.com/css2?family=Archivo:...');
@import "tailwindcss";
@source "../src/**/*.rs";

@theme { --color-ink: #201e1d; --color-accent: #ec3013; /* … */ }
```

Take colours from the tokens rather than hard-coding a hex — that is what keeps
the app and the design system in step.

`Cargo.toml` points cargo-leptos at it with `tailwind-input-file = "style/tailwind.css"`,
and cargo-leptos runs the pinned `tailwindcss` binary, pipes the result through
Lightning CSS and writes `target/site/pkg/goodie-never-deliver.css` — the file
`<Stylesheet id="leptos" .../>` in `src/app.rs` already links. `cargo leptos watch`
rebuilds the CSS when a `.rs` file changes, so new classes show up on save.

Notes:

- **No `tailwind.config.js`.** v4 is configured in CSS: `@theme` for design tokens,
  `@plugin "..."` for plugins, `@source` for extra scan paths. cargo-leptos only
  falls back to writing a v3-style JS config when it thinks Tailwind is v3 — which
  is why the dev shell exports `LEPTOS_TAILWIND_VERSION` with a `v` prefix.
- **Conditional classes.** `class:animate-pulse=move || { count.get() > 4 }` works,
  but keep the braces — a bare `>` inside an attribute value terminates the tag as
  far as the `view!` macro is concerned. `class=("animate-pulse", signal)` is the
  alternative for dynamic names.
- **Plugins** (`@tailwindcss/forms`, `@tailwindcss/typography`) come from npm:
  `npm i -D @tailwindcss/forms`, then `@plugin "@tailwindcss/forms";` in
  `style/tailwind.css`. That is what `nodejs` is in the shell for.
- Sass is still available if you want it: add `style-file = "style/main.scss"` back
  and cargo-leptos will compile it and concatenate it *before* the Tailwind output.

## Tests

```bash
cargo test --no-default-features --features ssr
```

Unit tests cover the money formatter, the index one-liner, the chip list and the
search/filter composition in `src/catalog.rs`.

The repository also carries the starter's Playwright scaffold in `end2end/`
(`cargo leptos end-to-end`, config in `[package.metadata.leptos]`). Its one
example spec still asserts the original template's "Welcome to Leptos" markup,
so it fails against this app until it is rewritten.

## Building for release

```bash
cargo leptos build --release
```

That produces two things:

1. the server binary at `target/release/goodie-never-deliver`
2. the site package at `target/site` (WASM, JS, CSS, and everything in `public/`)

To run it somewhere without the toolchain, copy both across, keeping the site
directory beside the binary:

```text
goodie-never-deliver
site/
```

and set:

```sh
export LEPTOS_OUTPUT_NAME="goodie-never-deliver"
export LEPTOS_SITE_ROOT="site"
export LEPTOS_SITE_PKG_DIR="pkg"
export LEPTOS_SITE_ADDR="127.0.0.1:3000"
export LEPTOS_RELOAD_PORT="3001"
export DATABASE_URL="postgres://user@host:5432/goodie"
```

`DATABASE_URL` is required: the binary refuses to start without it, and it runs
the migrations against that database before it binds the port.

## Licensing

Released into the public domain under the Unlicense — see `LICENSE`.
