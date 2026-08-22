<picture>
    <source srcset="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_Solid_White.svg" media="(prefers-color-scheme: dark)">
    <img src="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_RGB.svg" alt="Leptos Logo">
</picture>

# Leptos Axum Starter Template

This is a template for use with the [Leptos](https://github.com/leptos-rs/leptos) web framework and the [cargo-leptos](https://github.com/akesson/cargo-leptos) tool using [Axum](https://github.com/tokio-rs/axum).

## Creating your template repo

If you don't have `cargo-leptos` installed you can install it with

```bash
cargo install cargo-leptos --locked
```

Then run
```bash
cargo leptos new --git https://github.com/leptos-rs/start-axum
```

to generate a new project template.

```bash
cd goodie-never-deliver
```

to go to your newly created project.
Feel free to explore the project structure, but the best place to start with your application code is in `src/app.rs`.
Additionally, Cargo.toml may need updating as new versions of the dependencies are released, especially if things are not working after a `cargo update`.

## Running your project

```bash
cargo leptos watch
```

## Installing Additional Tools

By default, `cargo-leptos` uses `nightly` Rust, `cargo-generate`, and `sass`. If you run into any trouble, you may need to install one or more of these tools.

1. `rustup toolchain install nightly --allow-downgrade` - make sure you have Rust nightly
2. `rustup target add wasm32-unknown-unknown` - add the ability to compile Rust to WebAssembly
3. `cargo install cargo-generate` - install `cargo-generate` binary (should be installed automatically in future)
4. `npm install -g sass` - install `dart-sass` (should be optional in future
5. Run `npm install` in end2end subdirectory before test

## Compiling for Release
```bash
cargo leptos build --release
```

Will generate your server binary in target/release and your site package in target/site

## Testing Your Project
```bash
cargo leptos end-to-end
```

```bash
cargo leptos end-to-end --release
```

Cargo-leptos uses Playwright as the end-to-end test tool.
Tests are located in end2end/tests directory.

## Executing a Server on a Remote Machine Without the Toolchain
After running a `cargo leptos build --release` the minimum files needed are:

1. The server binary located in `target/server/release`
2. The `site` directory and all files within located in `target/site`

Copy these files to your remote server. The directory structure should be:
```text
goodie-never-deliver
site/
```
Set the following environment variables (updating for your project as needed):
```sh
export LEPTOS_OUTPUT_NAME="goodie-never-deliver"
export LEPTOS_SITE_ROOT="site"
export LEPTOS_SITE_PKG_DIR="pkg"
export LEPTOS_SITE_ADDR="127.0.0.1:3000"
export LEPTOS_RELOAD_PORT="3001"
```
Finally, run the server binary.

## Licensing

This template itself is released under the Unlicense. You should replace the LICENSE for your own application with an appropriate license if you plan to release it publicly.

## Development environment (Nix)

Everything the app builds against is pinned in `flake.nix` — no `cargo install`, no
system Postgres, no npm-installed Tailwind.

```bash
nix develop        # or: direnv allow   (an .envrc with `use flake` is included)
```

The shell provides:

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

Postgres holds `products` (see `migrations/0001_create_products.sql`): money as
integer cents, one row per object, a unique `slug` that is also the URL segment
at `/p/:slug`, and a nullable `note` for the buying desk's editorial.

Migrations run at startup, so a fresh cluster needs nothing but the dev shell:

```bash
pg-start                # initdb, start, create `goodie`
cargo leptos watch      # migrates, seeds, serves
```

Seed data is **committed**, not fetched at boot:

- `seed/dummyjson-products.json` — 194 products from
  [dummyjson.com/products](https://dummyjson.com/products), verbatim.
- `migrations/0002_seed_products.sql` — the insert statements, generated from
  that file by `scripts/generate-seed-sql.py`. Edit the payload or the
  generator, never the SQL.

```bash
python3 scripts/generate-seed-sql.py    # regenerate after changing the payload
sqlx migrate run                        # or just restart the app
```

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
