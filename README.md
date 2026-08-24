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

## Run it

The [latest release](https://github.com/ChunHou20c/goodie-store/releases/latest)
carries a prebuilt server for each platform, so there is nothing to compile.
Pick one, give it a Postgres, and open **http://127.0.0.1:3000**.

Whichever you pick, `DATABASE_URL` is the one thing you have to supply. The
server runs its own migrations on startup, so an empty database is fine — it
comes back with the 20 seed products already in it.

### Container

```bash
docker run --rm -p 3000:3000 \
  -e DATABASE_URL='postgres://user:pass@host:5432/goodie' \
  ghcr.io/chunhou20c/goodie-store:latest
```

Tagged `latest`, `v0.1.0-alpha` and `0.1.0-alpha`. It listens on `0.0.0.0:3000`.
To reach a Postgres running on the host, add `--network host` (Linux) or use
`host.docker.internal` in the URL (Docker Desktop).

Nothing to hand? This brings up the app and its database together:

```yaml
# compose.yaml — docker compose up
services:
  goodie:
    image: ghcr.io/chunhou20c/goodie-store:latest
    ports: ["3000:3000"]
    environment:
      DATABASE_URL: postgres://goodie:secret@db:5432/goodie
      ADMIN_EMAIL: admin@goodie.test
      ADMIN_PASSWORD: admin-dev-password
    depends_on:
      db: { condition: service_healthy }
  db:
    image: postgres:18-alpine
    environment:
      POSTGRES_USER: goodie
      POSTGRES_PASSWORD: secret
      POSTGRES_DB: goodie
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U goodie"]
      interval: 5s
      retries: 10
```

### Linux

```bash
curl -LO https://github.com/ChunHou20c/goodie-store/releases/download/v0.1.0-alpha/goodie-never-deliver-0.1.0-x86_64-linux.tar.gz
tar xzf goodie-never-deliver-0.1.0-x86_64-linux.tar.gz
cd goodie-never-deliver-0.1.0-x86_64-linux
DATABASE_URL='postgres://user:pass@host:5432/goodie' ./run.sh
```

### Windows

Download and unzip
[`goodie-never-deliver-0.1.0-x86_64-windows.zip`](https://github.com/ChunHou20c/goodie-store/releases/download/v0.1.0-alpha/goodie-never-deliver-0.1.0-x86_64-windows.zip),
then from that folder:

```bat
set DATABASE_URL=postgres://user:pass@host:5432/goodie
run.cmd
```

Neither archive needs a runtime installed — the Linux binary is statically
linked against musl, the Windows one imports nothing but system DLLs. Both
default to `127.0.0.1:3000`; set `LEPTOS_SITE_ADDR` to change it. `run.sh` and
`run.cmd` only exist to point the binary at the `site/` directory beside it, so
running the executable directly works too once you export the `LEPTOS_*`
variables below.

A `SHA256SUMS` file sits next to the archives on the release page:

```bash
curl -LO https://github.com/ChunHou20c/goodie-store/releases/download/v0.1.0-alpha/SHA256SUMS
sha256sum -c SHA256SUMS --ignore-missing
```

### Environment

The same variables apply to all three artifacts.

| | | |
| --- | --- | --- |
| `DATABASE_URL` | **required** | `postgres://user:pass@host:5432/goodie` — migrated on startup |
| `ADMIN_EMAIL` / `ADMIN_PASSWORD` | optional | upserts an admin account on startup; unset means no bootstrap |
| `LEPTOS_SITE_ADDR` | optional | listen address, default `127.0.0.1:3000` (`0.0.0.0:3000` in the container) |
| `LEPTOS_SITE_ROOT` / `LEPTOS_OUTPUT_NAME` / `LEPTOS_SITE_PKG_DIR` | preset | where the binary finds `site/`; the container and the two launcher scripts set these for you |

Set both admin variables to bootstrap an account, or neither — the server logs
which it did. They are read on every start, so changing `ADMIN_PASSWORD` and
restarting resets that account's password. Sign in at `/login` and the admin
console at `/admin` can import the rest of the catalogue from the upstream API.

The container takes environment the usual ways — `-e`, `--env-file`, compose, or
your orchestrator's secret mechanism. Two things are specific to it: everything
it presets is overridable (`-e LEPTOS_SITE_ADDR=0.0.0.0:8080` wins over the
baked-in default), and there is **no shell in the image**, so `docker run … sh
-c 'export …'`, an entrypoint wrapper and `docker exec … sh` are all
unavailable — variables have to arrive from outside. If `DATABASE_URL` is
missing the server exits non-zero before binding a port, so a misconfigured
deploy fails immediately rather than serving errors.

## Development

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

[BUILDING.md](BUILDING.md) covers building the release artifacts yourself with
Nix, and how a release is cut.

[ARCHITECTURE.md](ARCHITECTURE.md) covers the dev environment, the server/client
state split, the database and its migrations, the auth model, and the design
system.

Released into the public domain under the Unlicense — see `LICENSE`.
