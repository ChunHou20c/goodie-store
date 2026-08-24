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

        # Tell cargo-leptos which versions it is looking at: it skips the GitHub
        # release check, and — for Tailwind — the "v4" prefix is what switches it
        # to CSS-first config (no tailwind.config.js is written or passed with
        # --config). Formats must match cargo-leptos's own defaults, hence the
        # `v` and `version_` prefixes. The dev shell and the release build both
        # need these, so they live in one place.
        leptosVersions = {
          LEPTOS_TAILWIND_VERSION = "v${pkgs.tailwindcss_4.version}";
          LEPTOS_SASS_VERSION = pkgs.dart-sass.version;
          LEPTOS_WASM_OPT_VERSION = "version_${pkgs.binaryen.version}";
          LEPTOS_WASM_BINDGEN_VERSION = wasmBindgen.version;
        };

        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        pname = cargoToml.package.name;
        version = cargoToml.package.version;

        # Only what the build actually reads. Keeps `nix build` off the churn in
        # end2end/, the docs and the flake itself.
        src = pkgs.lib.fileset.toSource {
          root = ./.;
          fileset = pkgs.lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./rust-toolchain.toml
            ./src
            ./migrations
            ./public
            ./style
            ./seed # the importer's unit tests replay the committed payload
          ];
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        # `cargo leptos build --release` compiles the crate twice — the ssr
        # binary natively and the hydrate lib to wasm32 — then runs
        # wasm-bindgen, wasm-opt and tailwind over the result. Everything it
        # shells out to comes from nativeBuildInputs; nothing is downloaded.
        goodie-linux = rustPlatform.buildRustPackage (
          leptosVersions
          // {
            inherit pname version src;

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = leptosTooling ++ [
              pkgs.pkg-config
              # A C wrapper rather than a shell one: it keeps bash out of the
              # runtime closure, which matters for the container image.
              pkgs.makeBinaryWrapper
            ];

            # std's panic messages carry absolute source paths, and an
            # unremapped one pins the entire Rust toolchain — rustc, clippy,
            # rust-analyzer, both std targets — into the closure: 1.4 GiB of it.
            RUSTFLAGS = "--remap-path-prefix=${rustToolchain}=/rustc";

            # cargo-leptos writes a build cache under $HOME.
            preBuild = ''
              export HOME="$TMPDIR"
            '';

            buildPhase = ''
              runHook preBuild
              cargo leptos build --release
              runHook postBuild
            '';

            # The unit tests are server-side only; the default feature set has
            # neither `ssr` nor `hydrate` and would not link.
            checkPhase = ''
              runHook preCheck
              cargo test --release --no-default-features --features ssr
              runHook postCheck
            '';

            # The binary locates its assets through LEPTOS_SITE_ROOT, so the
            # site package ships in $out/share and the wrapper points at it.
            # --set-default, not --set: a deployment can still override any of
            # these from the environment.
            installPhase = ''
              runHook preInstall

              mkdir -p $out/bin $out/share/${pname}
              cp target/release/${pname} $out/bin/${pname}
              cp -r target/site $out/share/${pname}/site

              wrapProgram $out/bin/${pname} \
                --set-default LEPTOS_OUTPUT_NAME ${pname} \
                --set-default LEPTOS_SITE_ROOT $out/share/${pname}/site \
                --set-default LEPTOS_SITE_PKG_DIR pkg \
                --set-default LEPTOS_SITE_ADDR 0.0.0.0:3000 \
                --set-default LEPTOS_RELOAD_PORT 3001

              runHook postInstall
            '';

            meta = {
              description = "Full-stack shop on Leptos + Axum + Tailwind";
              mainProgram = pname;
              platforms = pkgs.lib.platforms.linux;
            };
          }
        );

        # ---- Standalone server builds ----------------------------------------
        #
        # `goodie-linux` above is a Nix-store package: it links against the
        # glibc in /nix/store and only runs where that path exists. The two
        # builds below are the ones that get uploaded to a release — each is a
        # statically linked server binary that runs on a bare machine, laid out
        # exactly as ARCHITECTURE.md describes a manual deployment: the
        # executable with `site/` beside it.
        #
        # Neither rebuilds the site package: WASM, JS, CSS and public/ are
        # byte-identical whatever the server runs on, so they are copied out of
        # the native build.
        muslTarget = "x86_64-unknown-linux-musl";
        winTarget = "x86_64-pc-windows-gnu";

        # One toolchain for both cross targets — the extra rust-std components
        # are small next to a second copy of rustc.
        rustToolchainCross = rustToolchain.override {
          targets = [
            "wasm32-unknown-unknown"
            muslTarget
            winTarget
          ];
        };

        rustPlatformCross = pkgs.makeRustPlatform {
          cargo = rustToolchainCross;
          rustc = rustToolchainCross;
        };

        ccFor = crossPkgs: "${crossPkgs.stdenv.cc}/bin/${crossPkgs.stdenv.cc.targetPrefix}";

        # Everything shared between the musl and the mingw build. `exe` is the
        # suffix the linker puts on the output; `systemDlls` is the allow-list
        # the Windows build checks its imports against (empty = skip).
        mkStandalone =
          {
            target,
            crossPkgs,
            exeSuffix ? "",
            extraRustflags ? "",
            launcher,
            launcherName,
            postInstall ? "",
          }:
          rustPlatformCross.buildRustPackage {
            pname = "${pname}-${target}";
            inherit version src;

            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ crossPkgs.stdenv.cc ];

            CARGO_BUILD_TARGET = target;

            # cc-rs (ring, and the other -sys crates) looks for the per-target
            # CC/AR pair rather than the plain ones.
            "CC_${builtins.replaceStrings [ "-" ] [ "_" ] target}" = "${ccFor crossPkgs}cc";
            "AR_${builtins.replaceStrings [ "-" ] [ "_" ] target}" = "${ccFor crossPkgs}ar";

            # Same path remap as the native build — see the note there. A
            # target-specific RUSTFLAGS replaces the generic one instead of
            # adding to it, so everything this target needs goes in one string.
            "CARGO_TARGET_${pkgs.lib.toUpper (builtins.replaceStrings [ "-" ] [ "_" ] target)}_RUSTFLAGS" =
              # `-C strip` rather than the strip hook: nixpkgs skips stripping
              # for cross builds, so the linker has to do it.
              "--remap-path-prefix=${rustToolchainCross}=/rustc -C strip=symbols ${extraRustflags}";
            "CARGO_TARGET_${pkgs.lib.toUpper (builtins.replaceStrings [ "-" ] [ "_" ] target)}_LINKER" =
              "${ccFor crossPkgs}cc";

            buildPhase = ''
              runHook preBuild
              cargo build --release --frozen \
                --target ${target} \
                --bin ${pname} --no-default-features --features ssr
              runHook postBuild
            '';

            # Cross-built tests cannot run on the builder; the same tests
            # already run in the native build, which gates this one.
            doCheck = false;

            installPhase = ''
              runHook preInstall

              mkdir -p $out
              cp target/${target}/release/${pname}${exeSuffix} $out/
              cp -r ${goodie-linux}/share/${pname}/site $out/site

              cp ${launcher} $out/${launcherName}
              chmod +x $out/${launcherName}

              ${postInstall}

              runHook postInstall
            '';

            meta.description = "Full-stack shop on Leptos + Axum + Tailwind (${target})";
          };

        # A distroless-style image: the server closure, plus CA certificates
        # for the admin importer's HTTPS call out to dummyjson, and /tmp
        # because tokio and reqwest both expect one. No shell, no package
        # manager — `docker run --entrypoint` will not find one.
        container = pkgs.dockerTools.buildLayeredImage {
          name = pname;
          tag = version;
          # Reproducible: without this the image gets the current timestamp and
          # its digest changes on every build.
          created = "1970-01-01T00:00:01Z";

          contents = [
            pkgs.cacert
            (pkgs.runCommand "tmp-dir" { } "mkdir -p $out/tmp")
          ];

          config = {
            Entrypoint = [ "${goodie-linux}/bin/${pname}" ];
            ExposedPorts."3000/tcp" = { };
            Env = [
              "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              # The wrapper sets these as defaults already; repeating them here
              # makes `docker inspect` describe the whole contract in one place.
              "LEPTOS_SITE_ADDR=0.0.0.0:3000"
            ];
            # DATABASE_URL is deliberately absent — it is deployment input, and
            # the server exits early and loudly when it is missing.
          };
        };

        # Both launchers set the three LEPTOS_* variables that tell the binary
        # where `site/` is, then hand over. DATABASE_URL is the caller's to set:
        # it is deployment input, and the server exits early and loudly without
        # it rather than guessing.
        linuxLauncher = pkgs.writeText "run.sh" ''
          #!/bin/sh
          # Start the server from the directory this script lives in.
          set -e
          cd "$(dirname "$0")"
          export LEPTOS_OUTPUT_NAME="${pname}"
          export LEPTOS_SITE_ROOT="site"
          export LEPTOS_SITE_PKG_DIR="pkg"
          export LEPTOS_SITE_ADDR="''${LEPTOS_SITE_ADDR:-127.0.0.1:3000}"
          exec ./${pname} "$@"
        '';

        windowsLauncher = pkgs.writeText "run.cmd" (
          builtins.replaceStrings [ "\n" ] [ "\r\n" ] ''
            @echo off
            rem Start the server from the directory this script lives in.
            cd /d "%~dp0"
            set "LEPTOS_OUTPUT_NAME=${pname}"
            set "LEPTOS_SITE_ROOT=site"
            set "LEPTOS_SITE_PKG_DIR=pkg"
            if "%LEPTOS_SITE_ADDR%"=="" set "LEPTOS_SITE_ADDR=127.0.0.1:3000"
            "%~dp0${pname}.exe" %*
          ''
        );

        goodie-static = mkStandalone {
          target = muslTarget;
          crossPkgs = pkgs.pkgsCross.musl64;
          launcher = linuxLauncher;
          launcherName = "run.sh";
          # musl already defaults to +crt-static; asking for the self-contained
          # link makes rustc supply its own libc objects instead of hunting for
          # them in the cross sysroot.
          extraRustflags = "-C target-feature=+crt-static -C link-self-contained=yes";

          # The whole point of this build is a binary with no runtime deps.
          postInstall = ''
            if ${pkgs.binutils}/bin/readelf -d $out/${pname} | grep -q NEEDED; then
              echo "${pname} (musl) is not statically linked" >&2
              exit 1
            fi
          '';
        };

        goodie-windows = mkStandalone {
          target = winTarget;
          crossPkgs = pkgs.pkgsCross.mingwW64;
          exeSuffix = ".exe";
          launcher = windowsLauncher;
          launcherName = "run.cmd";
          # rustc's windows-gnu spec links `-l:libpthread.a`, which lives in the
          # mingw pthreads package rather than in gcc's own sysroot.
          extraRustflags = "-L native=${pkgs.pkgsCross.mingwW64.windows.pthreads}/lib";

          # The exe must not need a mingw runtime beside it: libgcc, the pthread
          # shim and the unwinder are all linked statically, so every remaining
          # import should be one of Windows' own DLLs. Assert that here rather
          # than finding out on someone's laptop.
          postInstall = ''
            if ${pkgs.binutils}/bin/objdump -p $out/${pname}.exe \
                 | grep -i '^[[:space:]]*DLL Name:' \
                 | grep -Eiv '(kernel32|ntdll|advapi32|ws2_32|userenv|secur32|shell32|combase|msvcrt|bcrypt|bcryptprimitives|api-ms-win-)'; then
              echo "unexpected non-system DLL import in ${pname}.exe" >&2
              exit 1
            fi
          '';
        };

        # Release archives: what a CI run uploads, and what someone unpacks and
        # runs. Built here rather than in the workflow so `nix build` produces
        # the exact file the release carries.
        mkArchive =
          {
            drv,
            suffix,
            command,
          }:
          pkgs.runCommand "${pname}-${version}-${suffix}"
            {
              nativeBuildInputs = [
                pkgs.gnutar
                pkgs.gzip
                pkgs.zip
              ];
            }
            ''
              dir="${pname}-${version}-${suffix}"
              cp -rL ${drv} "$dir"
              chmod -R u+w "$dir"
              mkdir -p $out
              ${command}
            '';

        linux-archive = mkArchive {
          drv = goodie-static;
          suffix = "x86_64-linux";
          # --sort=name plus a fixed mtime and no gzip timestamp: the tarball is
          # byte-reproducible across runs.
          command = ''
            tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
              -cf - "$dir" | gzip -n -9 > $out/"$dir".tar.gz
          '';
        };

        windows-archive = mkArchive {
          drv = goodie-windows;
          suffix = "x86_64-windows";
          # `zip -r` walks the directory in readdir order, which is not stable
          # across filesystems — the payload would be identical but the archive
          # would not. Feeding it a sorted list over `-@` fixes the order, and
          # -X drops the uid/gid and timestamp extra fields.
          command = ''
            find "$dir" -exec touch -d @0 {} +
            find "$dir" | LC_ALL=C sort | zip -qX -9 $out/"$dir".zip -@
          '';
        };

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
        packages = {
          default = goodie-linux;
          ${pname} = goodie-linux;

          # Release artifacts.
          container = container;
          static = goodie-static;
          windows = goodie-windows;
          linux-archive = linux-archive;
          windows-archive = windows-archive;
        };

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

          env = leptosVersions // {
            # Postgres: cluster + socket inside the checkout, no system daemon.
            PGPORT = "5433";
            PGUSER = "postgres";
            PGDATABASE = "goodie";

            # Dev-only: the server upserts this admin on startup, so a `pg-reset`
            # still leaves an account that can reach the admin console. Nothing
            # here is a secret — the cluster is a throwaway on localhost.
            ADMIN_EMAIL = "admin@goodie.test";
            ADMIN_PASSWORD = "admin-dev-password";
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
