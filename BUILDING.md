# Building and releasing

You do not need any of this to *run* the app — the
[README](README.md#run-it) points at prebuilt artifacts. This is for building
them yourself, and for cutting a new release.

Everything here goes through `nix build`. There is no Dockerfile, no `rustup`,
no `cargo install`, and nothing is fetched at build time: the flake pins the
Rust nightly, `cargo-leptos`, `wasm-bindgen`, `wasm-opt`, `tailwindcss` and both
cross toolchains.

| | |
| --- | --- |
| [The outputs](#the-outputs) | what each `nix build` target produces |
| [How it fits together](#how-it-fits-together) | the three details that make it work |
| [CI](#ci) · [Cutting a release](#cutting-a-release) | the workflows, and the tag flow |

## The outputs

```bash
nix build .#default           # native package — this is the one that runs the tests
nix build .#container         # docker image, a loadable tar.gz
nix build .#linux-archive     # .tar.gz — static x86_64 Linux binary + site/ + run.sh
nix build .#windows-archive   # .zip    — x86_64 Windows .exe + site/ + run.cmd
nix run   .#                  # build and start it straight from the flake
```

All four come from the *same* build: one native compile produces the site
package, and the two standalone binaries are cross-compiled against it.

| Output | What it is |
| --- | --- |
| `.#default` | the native package — `bin/` plus `share/goodie-never-deliver/site`, with the `LEPTOS_*` variables baked in as wrapper defaults. This is the build that runs `cargo test`, and `nix run` starts it. It links against the glibc in `/nix/store`, so it runs where Nix does |
| `.#container` | a `dockerTools.buildLayeredImage` tarball: the package's closure, CA certificates, `/tmp`, and nothing else — no shell, no package manager |
| `.#static` / `.#linux-archive` | `x86_64-unknown-linux-musl`, statically linked, plus `site/` and a `run.sh` |
| `.#windows` / `.#windows-archive` | `x86_64-pc-windows-gnu` cross-compiled through `pkgsCross.mingwW64`, plus `site/` and a CRLF `run.cmd` |

Both archives are byte-reproducible: fixed mtimes, sorted entries, no gzip
timestamp. Two builds of the same commit produce the same checksum, which is
what makes the `SHA256SUMS` on a release worth checking.

The archives unpack to the layout
[ARCHITECTURE.md](ARCHITECTURE.md#building-for-release) describes for a manual
deployment — the executable with `site/` beside it.

## How it fits together

**The site package is built once.** WASM, JS, CSS and `public/` do not depend on
the server's target, so the two cross builds copy `site/` out of the native
package rather than running `cargo leptos` again. Only the `ssr` binary is
cross-compiled, with `cargo build --no-default-features --features ssr`.

**Panic locations pin the toolchain.** `std`'s panic messages embed absolute
paths into the binary, and an unremapped one keeps *the entire Rust toolchain* —
rustc, clippy, rust-analyzer, both `rust-std` targets — alive in the closure.
The build passes `--remap-path-prefix`, which takes the runtime closure from
1.4 GiB to 73 MiB, and the container image with it.

**Both standalone binaries are checked for runtime dependencies at build time.**
The musl one fails the build if `readelf -d` reports any `NEEDED` entry; the
Windows one fails if `objdump -p` shows an import that is not a Windows system
DLL. That is what makes "unzip and run" true rather than hopeful — `ring`,
`argon2`, `sqlx` and rustls are all pure Rust or statically linked C, and
`reqwest` uses rustls with `webpki-roots`, so there is no OpenSSL and no system
certificate store to find.

## CI

`.github/workflows/ci.yml` runs on pushes to `main`, on pull requests, and on
demand. It evaluates the flake, builds the native package (which is what runs
the unit tests), builds the three release artifacts, and uploads them. A second
job then loads the container image, points it at a throwaway Postgres service
container, and checks that it serves the SSR shell, the hydration bundle and the
stylesheet — a missing or misplaced `site/` shows up there rather than in
production.

Both workflows cache the Nix store between runs with
`nix-community/cache-nix-action`, keyed on `flake.lock`, `Cargo.lock` and
`rust-toolchain.toml`. A cold run compiles the crate three times over and takes
about twenty minutes; a warm one is a couple of minutes.

## Cutting a release

Push a `v*` tag:

```bash
git tag v0.2.0 && git push origin v0.2.0
```

`.github/workflows/release.yml` then repeats the build — including the tests, so
a release cannot be cut from a commit whose tests fail — and publishes:

- the container to `ghcr.io/<owner>/<repo>`, tagged with the tag, the
  tag without its leading `v`, and `latest`;
- a GitHub release carrying both archives and a `SHA256SUMS`.

The image name follows the GitHub repository name, so renaming the repo renames
the image on the next release. The archive *filenames* carry the version from
`Cargo.toml`, which is independent of the tag — `v0.1.0-alpha` shipped
`goodie-never-deliver-0.1.0-…`. Bump `Cargo.toml` when you want those to agree.

`workflow_dispatch` runs the same job against an existing tag if a release needs
rebuilding.
