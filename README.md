# Goodie Store

A full-stack shop demo: browse a catalogue, filter and sort it, fill a bag, sign
in, and — as an admin — import more products from an upstream API. Every screen
is server-rendered and hydrated, on a design system ported from a Claude Design
canvas.

## Stack

| | |
| --- | --- |
| **UI** | [Leptos](https://github.com/leptos-rs/leptos) 0.8 — SSR + hydration, one Rust codebase compiled to native and WASM |
| **Server** | [Axum](https://github.com/tokio-rs/axum) 0.8, with Leptos server functions over `POST /api/*` |
| **Data** | Postgres 18 via [sqlx](https://github.com/launchbadge/sqlx) 0.8 — runtime-checked queries, migrations embedded in the binary |
| **Auth** | argon2id passwords, opaque session tokens in an `HttpOnly` cookie, two roles |
| **Styling** | Tailwind CSS v4, design tokens in `@theme`, compiled by `cargo-leptos` |
| **Toolchain** | a Nix flake pins everything: Rust nightly + `wasm32` (rust-overlay), `cargo-leptos`, `wasm-bindgen`, `tailwindcss`, Postgres, Node |

## Quick start

Everything the project needs is pinned in `flake.nix` — nothing to
`cargo install`, `npm install -g`, or `rustup add`, and no system Postgres.

```bash
nix develop           # or: direnv allow
pg-start              # initdb on first run, start Postgres, create `goodie`
cargo leptos watch    # migrate, seed, build and serve
```

Then open **http://127.0.0.1:3000**. That is the whole setup on a clean
checkout; `cargo leptos watch` rebuilds the server, the WASM bundle and the CSS
on save.

The database starts with 20 products. To see the admin side, sign in at
[/login](http://127.0.0.1:3000/login) with the dev credentials the shell
exports — `admin@goodie.test` / `admin-dev-password` — and import the rest of
the catalogue from `/admin`.

```bash
pg-stop        # stop the cluster
pg-reset       # throw it away; pg-start rebuilds it from the migrations
cargo test --no-default-features --features ssr
cargo leptos build --release
```

## More

[ARCHITECTURE.md](ARCHITECTURE.md) covers the dev environment, the server/client
state split, the database and its migrations, the auth model, the design system,
and the release build.

Released into the public domain under the Unlicense — see `LICENSE`.
