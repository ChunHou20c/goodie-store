{
  description = "goodie-never-deliver — full-stack shop on Leptos + Axum + Tailwind";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        # Channel + components + targets all come from ./rust-toolchain.toml, so
        # `cargo`, `rustc`, `clippy` and rust-analyzer agree with each other and
        # the wasm32 target is always present.
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        # The wasm-bindgen CLI must match the `wasm-bindgen` crate in Cargo.lock
        # exactly (it refuses to run on a schema mismatch). Bump both together:
        #   1. change the attribute below
        #   2. cargo update -p wasm-bindgen --precise <version>
        wasmBindgen = pkgs.wasm-bindgen-cli_0_2_126;

        postgresql = pkgs.postgresql_18;

        # cargo-leptos is built with the `no_downloads` feature in nixpkgs, so it
        # never fetches toolchain binaries at runtime — it picks up sass,
        # tailwindcss, wasm-opt and wasm-bindgen from PATH instead. All four have
        # to be in the shell for `cargo leptos build` to work.
        leptosTooling = [
          pkgs.cargo-leptos
          wasmBindgen
          pkgs.binaryen # wasm-opt
          pkgs.dart-sass # sass
          pkgs.tailwindcss_4 # tailwindcss
        ];

        # Cluster lives in ./.pg so it is per-checkout and disposable.
        pgScript =
          name: body:
          pkgs.writeShellApplication {
            inherit name;
            runtimeInputs = [ postgresql ];
            text = ''
              if [ -z "''${PGDATA:-}" ]; then
                echo "${name}: no PGDATA — run this inside 'nix develop'." >&2
                exit 1
              fi
            ''
            + body;
          };

        pg-start = pgScript "pg-start" ''
          mkdir -p "$PGHOST"
          if [ ! -d "$PGDATA" ]; then
            echo "pg-start: initialising cluster in $PGDATA"
            initdb --username="$PGUSER" --auth=trust --encoding=UTF8 --no-locale >/dev/null
          fi
          if pg_ctl status >/dev/null 2>&1; then
            echo "pg-start: already running on $PGHOST:$PGPORT"
            exit 0
          fi
          pg_ctl start -w -l "$PGHOST/postgres.log" \
            -o "-k $PGHOST -h 127.0.0.1 -p $PGPORT"
          if ! psql -lqtA | cut -d'|' -f1 | grep -qx "$PGDATABASE"; then
            createdb "$PGDATABASE"
            echo "pg-start: created database $PGDATABASE"
          fi
          echo "pg-start: $DATABASE_URL"
        '';

        pg-stop = pgScript "pg-stop" ''
          pg_ctl stop -m fast
        '';

        pg-reset = pgScript "pg-reset" ''
          pg_ctl status >/dev/null 2>&1 && pg_ctl stop -m immediate
          rm -rf "$PGDATA" "$PGHOST/postgres.log"
          echo "pg-reset: cluster removed — run pg-start to recreate it"
        '';
      in
      {
        devShells.default = pkgs.mkShell {
          name = "goodie-never-deliver";

          packages = [
            rustToolchain
            pkgs.cargo-generate
            pkgs.leptosfmt

            postgresql
            pkgs.sqlx-cli
            pg-start
            pg-stop
            pg-reset

            pkgs.nodejs_24 # tailwind plugins, playwright e2e, npx

            # Native deps for the usual axum/sqlx/reqwest stack.
            pkgs.pkg-config
            pkgs.openssl
          ]
          ++ leptosTooling;

          env = {
            # Postgres: cluster + socket inside the checkout, no system daemon.
            PGPORT = "5433";
            PGUSER = "postgres";
            PGDATABASE = "goodie";

            # Dev-only: the server upserts this admin on startup, so a `pg-reset`
            # still leaves an account that can reach the admin console. Nothing
            # here is a secret — the cluster is a throwaway on localhost.
            ADMIN_EMAIL = "admin@goodie.test";
            ADMIN_PASSWORD = "admin-dev-password";
            # Tell cargo-leptos which versions it is looking at: it skips the
            # GitHub release check, and — for Tailwind — the "v4" prefix is what
            # switches it to CSS-first config (no tailwind.config.js is written
            # or passed with --config). Formats must match cargo-leptos's own
            # defaults, hence the `v` and `version_` prefixes.
            LEPTOS_TAILWIND_VERSION = "v${pkgs.tailwindcss_4.version}";
            LEPTOS_SASS_VERSION = pkgs.dart-sass.version;
            LEPTOS_WASM_OPT_VERSION = "version_${pkgs.binaryen.version}";
            LEPTOS_WASM_BINDGEN_VERSION = wasmBindgen.version;
          };

          shellHook = ''
            root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
            export PGDATA="$root/.pg/data"
            export PGHOST="$root/.pg"
            # TCP rather than the socket: psql picks up PGHOST on its own, and
            # sqlx wants a URL it can parse without a host= parameter.
            export DATABASE_URL="postgres://$PGUSER@127.0.0.1:$PGPORT/$PGDATABASE"

            lock_wb="$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p;q;}' \
              "$root/Cargo.lock" 2>/dev/null || true)"
            if [ -n "$lock_wb" ] && [ "$lock_wb" != "${wasmBindgen.version}" ]; then
              echo "warning: Cargo.lock pins wasm-bindgen $lock_wb but the CLI is ${wasmBindgen.version}."
              echo "         run: cargo update -p wasm-bindgen --precise ${wasmBindgen.version}"
            fi

            echo "goodie-never-deliver dev shell"
            echo "  $(rustc --version)  |  cargo-leptos ${pkgs.cargo-leptos.version}  |  wasm-bindgen ${wasmBindgen.version}"
            echo "  tailwindcss ${pkgs.tailwindcss_4.version}  |  node $(node --version)  |  postgres ${postgresql.version}"
            echo "  pg-start / pg-stop / pg-reset   cargo leptos watch"
          '';
        };

        formatter = pkgs.nixfmt-tree;
      }
    );
}
