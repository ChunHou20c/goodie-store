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

## Running a release build

Three artifacts come out of the flake, and CI builds all three on every push.
They are the *same* build — one native compile produces the site package, and
the two standalone binaries are cross-compiled from it.

```bash
nix build .#container         # docker image, loadable tar.gz
nix build .#linux-archive     # .tar.gz — static x86_64 Linux binary + site/
nix build .#windows-archive   # .zip    — x86_64 Windows .exe + site/
nix run   .#                  # build and start it straight from the flake
```

All of them need one thing from you: `DATABASE_URL`, pointing at a Postgres
instance. The server runs its own migrations against it on startup, so an empty
database is fine — it comes back with the 20 seed products in it.

### Container

```bash
docker load -i result                       # or: docker pull ghcr.io/<owner>/goodie-never-deliver
docker run --rm -p 3000:3000 \
  -e DATABASE_URL='postgres://user:pass@host:5432/goodie' \
  goodie-never-deliver:0.1.0
```

It listens on `0.0.0.0:3000` and has no shell in it — the entrypoint is the
server. To reach a Postgres running on the host, add `--network host` (Linux) or
use `host.docker.internal` in the URL (Docker Desktop).

### Linux and Windows archives

Unpack, and run the launcher beside the binary:

```bash
tar xzf goodie-never-deliver-0.1.0-x86_64-linux.tar.gz
cd goodie-never-deliver-0.1.0-x86_64-linux
DATABASE_URL='postgres://user:pass@host:5432/goodie' ./run.sh
```

```bat
rem after unzipping goodie-never-deliver-0.1.0-x86_64-windows.zip
set DATABASE_URL=postgres://user:pass@host:5432/goodie
run.cmd
```

Both default to `127.0.0.1:3000`; set `LEPTOS_SITE_ADDR` to change it. The
launcher exists because the binary finds its assets through `LEPTOS_SITE_ROOT` —
running the executable directly works too, as long as you export the variables
[ARCHITECTURE.md](ARCHITECTURE.md#building-for-release) lists. Neither binary
needs a runtime installed: the Linux one is statically linked against musl, and
the Windows one imports nothing but system DLLs.

### Optional environment

| | |
| --- | --- |
| `ADMIN_EMAIL` / `ADMIN_PASSWORD` | upserts an admin account on startup; unset means no bootstrap |
| `LEPTOS_SITE_ADDR` | listen address, default `127.0.0.1:3000` (`0.0.0.0:3000` in the container) |

## Releasing

CI runs on every push: it builds all three artifacts, then starts the container
against a throwaway Postgres and checks that it serves the app shell, the
hydration bundle and the stylesheet. Pushing a `v*` tag additionally publishes
the container to GHCR and attaches the archives to a GitHub release.

```bash
git tag v0.1.0 && git push origin v0.1.0
```

## More

[ARCHITECTURE.md](ARCHITECTURE.md) covers the dev environment, the server/client
state split, the database and its migrations, the auth model, and the design
system.

Released into the public domain under the Unlicense — see `LICENSE`.
