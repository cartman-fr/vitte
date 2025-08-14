
# Arborescence canonique (source de vérité) — mise à jour

```
.
├─ Cargo.toml
├─ .rustfmt.toml
├─ clippy.toml
├─ deny.toml
├─ rust-toolchain.toml
├─ util.vitte
│
├─ modules/
│  ├─ README.md
│  ├─ exports.vitte
│  ├─ log.vitte
│  ├─ config.vitte
│  ├─ metrics.vitte
│  ├─ taskpool.vitte
│  ├─ cache.vitte
│  ├─ kvstore.vitte
│  ├─ http_client.vitte
│  ├─ eventbus.vitte
│  ├─ feature_flags.vitte
│  ├─ validate.vitte
│  ├─ cli.vitte
│  ├─ channel.vitte
│  ├─ scheduler.vitte
│  ├─ retry.vitte
│  ├─ rate_limiter.vitte
│  ├─ idgen.vitte
│  ├─ uuid.vitte
│  ├─ random.vitte
│  ├─ csv.vitte
│  ├─ ini.vitte
│  ├─ yaml_lite.vitte
│  ├─ checksum.vitte
│  ├─ rle.vitte
│  ├─ pool.vitte
│  ├─ prioq.vitte
│  ├─ fs_atomic.vitte
│  ├─ supervisor.vitte
│  ├─ plugin.vitte
│  ├─ migrate.vitte
│  ├─ tracing.vitte
│  ├─ pagination.vitte
│  ├─ result_ext.vitte
│  ├─ stringx.vitte
│  ├─ mathx.vitte
│  └─ graph.vitte
│
├─ desktop/
│  ├─ README.md
│  ├─ main.vitte
│  ├─ gtk_stub.c
│  ├─ gtk_real.c
│  ├─ qt_stub.cpp
│  └─ qt_real.cpp
│
├─ embedded-blink/
│  └─ main.vitte
│
├─ hello/
│  ├─ vitte.toml
│  └─ src/
│     └─ main.vit
│
├─ hello-vitte/
│  └─ main.vitte
│
├─ kernel/
│  ├─ README.md
│  ├─ armv7em/
│  │  ├─ kmain.vitte
│  │  ├─ start.S
│  │  └─ linker.ld
│  └─ x86_64/
│     ├─ kmain.vitte
│     ├─ start.S
│     └─ linker.ld
│
├─ wasm-add/
│  └─ main.vitte
│
├─ web-echo/
│  ├─ README.md
│  ├─ main.vitte
│  └─ middleware.vitt
│
├─ worker-jobs/
│  └─ main.vitte
│
├─ crates/
│  ├─ vitte-cli/
│  │  ├─ Cargo.toml
│  │  └─ src/main.rs
│  ├─ vitte-compiler/
│  │  ├─ Cargo.toml
│  │  └─ src/lib.rs
│  ├─ vitte-core/
│  │  ├─ Cargo.toml
│  │  └─ src/lib.rs
│  ├─ vitte-runtime/
│  │  ├─ Cargo.toml
│  │  └─ src/lib.rs
│  ├─ citte/
│  │  ├─ Cargo.toml
│  │  └─ src/lib.rs
│  ├─ stdlib/
│  │  ├─ Cargo.toml
│  │  └─ src/lib.rs
│  ├─ vitte-tools/
│  │  ├─ Cargo.toml
│  │  └─ src/lib.rs
│  ├─ vitte-vm/
│  │  ├─ Cargo.toml
│  │  └─ src/lib.rs
│  └─ README.md
│
├─ .github/
│  ├─ workflows/
│  │  ├─ ci.yml
│  │  ├─ release.yml
│  │  └─ pages-docs.yml
│  ├─ dependabot.yml
│  ├─ CODEOWNERS
│  ├─ ISSUE_TEMPLATE/
│  └─ PULL_REQUEST_TEMPLATE.md
│
└─ tools/
   ├─ vitc/               # utilitaire (script/binaire)
   ├─ vitcc/              # utilitaire (script/binaire)
   ├─ vit-pm/             # gestionnaire de paquets/projets (script/binaire)
   ├─ vitte-bench/        # bench suite
   ├─ vitte-doc/          # génération doc
   ├─ vitte-fmt/          # formateur
   ├─ vitte-profile/      # profiler
   ├─ vitx/               # outil avancé X
   └─ vitxx/              # outil avancé XX