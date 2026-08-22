# Goodie Store — implementation notes

Detail behind the demo: how the dev environment is pinned, what is server state
and what is not, the database and its migrations, the auth model, the design
system, and how it is tested and built. The [README](README.md) is the quick
start.

| | |
| --- | --- |
| [Development environment (Nix)](#development-environment-nix) | the pinned toolchain, Postgres helpers, wasm-bindgen pinning |
| [The app](#the-app--goodie-store) | the screens, and the server/client state split |
| [The database](#the-database) | schema, migrations, seed data |
| [Accounts and the admin console](#accounts-and-the-admin-console) | roles, sessions, the admin-only import |
| [Styling](#styling-tailwind-css-v4) | design tokens and the Tailwind v4 setup |
| [Tests](#tests) · [Building for release](#building-for-release) | what runs, and what ships |

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

## The app — Goodie Store

The four screens implement `Kessel Shop.dc.html` from the *Mobile shopping app
design* Claude Design project, on its **Modernist** design system.

| Route | Screen |
| --- | --- |
| `/` | Cover story, then the first few objects on the shelf |
| `/search` | Filter chips, cycling sort, two-column results |
| `/p/:id` | Photograph, price, spec table, the desk's note |
| `/bag` | Lines with steppers, totals, checkout |
| `/login` | Sign in, register, sign out — doubles as the account screen |
| `/admin` | The import console, admin only |

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

The catalogue and your account come from Postgres. The bag, the saved list, the
query, the chips and the sort all live in the tab — they never reach the server,
and reloading the page empties them.

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

- `seed/dummyjson-products.json` — all 194 products from
  [dummyjson.com/products](https://dummyjson.com/products), verbatim.
- `migrations/0002_seed_products.sql` — insert statements for the **first 20**
  of them, generated from that file by `scripts/generate-seed-sql.py`. Edit the
  payload or the generator, never the SQL. The inserts end in
  `on conflict (id) do nothing`, so applying them twice is safe.

A fresh database therefore starts with a browsable shelf of 20, and the
remaining 174 are pulled in from the [admin console](#importing-products-admin-only)
— which is the point of having an importer at all. Change `SEED_LIMIT` in the
generator and re-run it if you want a fuller starting catalogue.

To refresh it:

```bash
FIELDS=id,title,description,category,price,discountPercentage,rating,stock,brand,\
sku,weight,dimensions,warrantyInformation,shippingInformation,availabilityStatus,\
returnPolicy,minimumOrderQuantity,tags,thumbnail,images
curl -s "https://dummyjson.com/products?limit=0&select=$FIELDS" \
    -o seed/dummyjson-products.json
python3 scripts/generate-seed-sql.py     # writes the first SEED_LIMIT rows
pg-reset && pg-start          # the seed migration's checksum has changed
cargo leptos watch
```

That last step is the checksum rule above: regenerating `0002` after it has been
applied invalidates it, so the database has to start clean.

Even at 20 rows the seed brings real categories (beauty, fragrances, furniture,
groceries), prices from $1.99 to $2,499.99, mixed availability and product
photography, so the chip row, the empty states and the price sort all exercise
real data. Importing the rest widens that to 24 categories and $0.79–$36,999.99.

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
"Why we stock it" panel only appears when a row has a `note`; nothing in the seed
writes one.

## Accounts and the admin console

Two roles, `user` and `admin` (a Postgres enum), in `users`; sessions in
`sessions`. Both arrive with `migrations/0003_create_users.sql`.

### How a session works

Signing in mints 32 random bytes, base64url-encoded, and returns them in a
cookie:

```
goodie_session=<token>; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000
```

The database stores only `sha256(token)`, so a dump of `sessions` cannot be
replayed as a live login. `SameSite=Lax` also keeps the cookie off cross-site
POSTs, which is the CSRF story for now. Set `APP_SECURE_COOKIES=1` in production
to add `Secure` (it is off by default so plain-http localhost works).

Passwords are argon2id with the crate's default parameters. A wrong email and a
wrong password give the same message, so the form does not confirm who has an
account.

`leptos_axum` puts the request `Parts` and `ResponseOptions` in context for both
page renders and server-function calls, so `src/auth.rs` reads and writes that
cookie the same way in either — there is no axum middleware in the path.

### Server functions

| Endpoint | Who | Does |
| --- | --- | --- |
| `POST /api/sign_up` | anyone | creates a `user`, opens a session |
| `POST /api/sign_in` | anyone | opens a session |
| `POST /api/sign_out` | anyone | deletes the session row, clears the cookie |
| `POST /api/current_user` | anyone | the signed-in user, or none |
| `POST /api/import_products` | **admin only** | pulls products from dummyjson into the database |

`/login` is also the account screen: signed in it shows who you are, a sign-out
button, and — for admins — the link to `/admin`.

### The first admin

If `ADMIN_EMAIL` and `ADMIN_PASSWORD` are both set, the server upserts that
account as an admin on startup; unset, it skips the step and says so in the log.
The dev shell exports throwaway values (`admin@goodie.test`), so `pg-reset`
always leaves you with a working admin. Set real ones in production, or omit them
and promote an account by hand.

### Importing products (admin only)

`/admin` fetches a slice of the upstream catalogue **server-side** — the browser
never talks to dummyjson — and upserts it:

- rows are matched on the upstream id, so re-running a range refreshes rather
  than duplicates, and the report distinguishes inserted from refreshed;
- `note` is never overwritten: it is editorial, and ours;
- `limit` is clamped to 100; slug collisions take the product id as a suffix.

Authorization is `require_admin()` inside the server function. Hiding `/admin`
from the nav is presentation, not a control — a signed-out or non-admin `POST`
to the endpoint is refused on its own merits.

The upstream→row mapping now exists twice: in Rust (`src/catalog.rs`, canonical)
and in Python (`scripts/generate-seed-sql.py`, only for regenerating the offline
seed). `cargo test` replays the committed payload through the Rust path and
compares it against what the generator wrote, so the two cannot drift silently.

### Not done yet

Password reset, email verification, rate limiting on sign-in, and a sweeper for
expired sessions (they stop authenticating on time, but the rows stay). User
management and an activity log are the next admin slices.

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

Unit tests cover the money formatter, the shelf one-liner, the chip list and the
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
