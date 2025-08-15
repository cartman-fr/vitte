# Getting Started avec **Vitte** (getting-started.md • ultra complet)

> _“On forge un langage comme une épée : tranchant, équilibré, fiable. À toi de jouer.”_  
> **Édition** : 2025 • **Toolchain minimal** : ≥ 0.6.0 • Plateformes : 🖥 desktop, 🛠 server, 🔌 embedded, 🌐 WASM

---

## Sommaire

1. [TL;DR (3 commandes)](#tldr-3-commandes)  
2. [Prérequis](#prérequis)  
3. [Installer Vitte](#installer-vitte) — binaire, Cargo, depuis les sources  
4. [Créer un projet](#créer-un-projet) — `vitpm new`, `vitte.toml`  
5. [Compiler & exécuter](#compiler--exécuter) — VM, LLVM, modes, flags  
6. [Structure d’un projet](#structure-dun-projet) — layout minimal & monorepo  
7. [Exemples essentiels](#exemples-essentiels) — CLI, fichiers, HTTP, concurrency  
8. [Tests & Qualité](#tests--qualité) — unit/integration, fmt, lints  
9. [Bench & Profiling](#bench--profiling)  
10. [Debug & Logs](#debug--logs) — `VITTE_LOG`, tracing  
11. [Packaging & Dépendances](#packaging--dépendances) — `vitpm`  
12. [Cibles : WASM, Embedded, Desktop](#cibles--wasm-embedded-desktop)  
13. [CI/CD GitHub Actions](#cicd-github-actions)  
14. [Dépannage (FAQ)](#dépannage-faq)  
15. [Bonnes pratiques & perfs](#bonnes-pratiques--perfs)  
16. [Aller plus loin](#aller-plus-loin)

---

## TL;DR (3 commandes)

```bash
# 1) Crée un projet
vitpm new hello-vitte && cd hello-vitte

# 2) Compile et exécute (backend par défaut)
vitc build && vitc run

# 3) Format + tests
vitte-fmt . && ./scripts/test.sh
```

---

## Prérequis

- **OS** : Linux, macOS (Intel/Apple Silicon), Windows x64, BSD  
- **Outils recommandés** :
  - Git ≥ 2.40
  - LLVM (si backend natif)
  - Make/CMake (selon outils)
  - Python 3.x (scripts auxiliaires)
  - **Rust** (si build from source) : `rustup`, `cargo`, `clippy`, `rustfmt`
- **Éditeur** : VS Code + extension syntaxe Vitte (voir `editor-plugins/vscode`)  

---

## Installer Vitte

### A) Binaire précompilé (le plus rapide)
```bash
# Exemple Linux x86_64
tar -xzf vitte-0.6.0-linux-x86_64.tar.gz
sudo mv vitte vitc vitpm vitte-fmt vitte-bench /usr/local/bin/
vitte --version
```

### B) Via Cargo (si le projet inclut des crates CLI)
```bash
rustup toolchain install stable
cargo install --path ./crates/vitte-cli
vitte --help
```

### C) Depuis les sources (monorepo)
```bash
# À la racine du repo
./scripts/ci_check.sh       # lint + fmt + build rapide
./scripts/build_all.sh      # build complet (VM + LLVM + tools)
./scripts/test_all.sh       # test suite
```

---

## Créer un projet

### `vitpm new` (application)
```bash
vitpm new my-app
cd my-app
tree -a
```

Layout minimal :
```
my-app/
├── vitte.toml
└── src/
   └── main.vit
```

### `vitte.toml` (manifest)
```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2025"

[build]
# backend = "vm" | "llvm" | "jit"
backend = "vm"
opt-level = "debug"   # "release" pour prod

[dependencies]
# stdlib incluse par défaut ; activer modules optionnels si packagés
http_client = { version = "0.1", optional = true }
retry = { version = "0.1", optional = true }

[features]
default = []
net = ["http_client", "retry"]
```

---

## Compiler & exécuter

### Build & Run
```bash
vitc build               # compile
vitc run                 # build + run
vitc run -- args --foo   # arguments passés au binaire
```

### Profils & backends
```bash
vitc build -p release         # -p/--profile: release ou debug
vitc build --backend llvm     # overrides manifest
vitc build --backend vm
```

### Flags utiles
```bash
vitc build --emit-asm               # dump assembleur (backend natif)
vitc build --emit-ir                # IR intermédiaire
vitc test                           # alias scripts/test.sh si configuré
vitte-fmt .                         # format tout
```

---

## Structure d’un projet

### Minimal (app)
```
.
├── vitte.toml
└── src/
   └── main.vit
```

### Monorepo (extrait)
```
.
├── crates/          # compiler, runtime, vm, stdlib, tools
├── modules/         # log, config, http_client, kvstore, ...
├── tools/           # vitc, vitpm, vitte-fmt, vitte-bench, ...
├── examples/        # hello, web-echo, wasm-add, kernel/*
├── docs/            # index.md, language-spec.md, stdlib.md, ...
├── scripts/         # build_all.sh, test_all.sh, gen_docs.sh, ...
└── .github/workflows/ci.yml
```

---

## Exemples essentiels

### 1) Hello
```vitte
do main() { print("Hello, Vitte!") }
```

### 2) CLI (args)
```vitte
use cli

do main() -> i32 {
  let args = cli::parse()
  if cli::has(args, "--help") { print("Usage: app --name X"); return 0 }
  let name = cli::get(args, "--name").unwrap_or("world")
  print("Hello, " + name)
  0
}
```

### 3) Fichiers (atomique)
```vitte
use fs

do save_atomic(p: str, data: []u8) -> Result[Unit, FsError] {
  fs::write_atomic(p, data)?
  Ok(())
}
```

### 4) HTTP + Retry
```vitte
use http_client; use retry; use string

do fetch_with_retry(url: str) -> Result[String, str] {
  let policy = retry::exponential_backoff(max_retries: 4, base_ms: 120, jitter: true)
  match retry::run(policy, || http_client::get(url)) {
    Ok(r)  => Ok(string::from_bytes(r.body)),
    Err(e) => Err("net error: " + to_string(e))
  }
}
```

### 5) Concurrence (threads + channels)
```vitte
use thread; use channel

do sum_parallel(xs: Vec[i32]) -> i64 {
  let (tx, rx) = channel::channel
  let mid = xs.len() / 2
  let left = thread::spawn({
    let mut s = 0
    for x in xs[0..mid] { s += x as i64 }
    tx.send(s)
  })
  let right = thread::spawn({
    let mut s = 0
    for x in xs[mid..] { s += x as i64 }
    tx.send(s)
  })
  let a = rx.recv().unwrap()
  let b = rx.recv().unwrap()
  left.join(); right.join()
  a + b
}
```

### 6) Async (preview)
```vitte
async do fetch_status(url: str) -> i32 {
  let r = await http_client::get(url)
  r.status
}
```

---

## Tests & Qualité

### Tests unitaires (attribut preview `@test`)
```vitte
@test
do parse_number() {
  assert(string::to_i32("42").unwrap()==42, "bad parse")
}
```

Exécuter :
```bash
./scripts/test.sh      # ou vitc test si mappé
./scripts/test_all.sh  # suite complète monorepo
```

### Format & lints
```bash
vitte-fmt .
./scripts/lint.sh      # clippy/deny selon config
```

---

## Bench & Profiling
```bash
vitte-bench run benchmarks/micro
vitte-profile --bin target/release/my-app
```

Conseils : exécuter sur machine “calme”, épingler l’affinité CPU, répéter 10x, exporter CSV.

---

## Debug & Logs

- **Niveaux** : `trace, debug, info, warn, error`  
- **Env** : `VITTE_LOG=debug`  
- **Trace** (preview) : spans & context

```bash
VITTE_LOG=info vitc run
```

Dans le code :
```vitte
use log

do main(){
  log::info("starting")
  // ...
}
```

---

## Packaging & Dépendances

### Créer une lib
```bash
vitpm new my-lib --lib
```

### Dépendre d’un module
```toml
[dependencies]
stringx = "0.1"
yaml_lite = { version = "0.1", optional = true }
```

### Workspaces (monorepo)
```toml
[workspace]
members = ["crates/*", "tools/*", "examples/*"]
```

---

## Cibles : WASM, Embedded, Desktop

### WASM (limitations : pas de `fs/process`, réseau restreint)
```bash
vitc build --backend vm --target wasm
```
Code :
```vitte
do main(){ print("Hello from WASM") }
```

### Embedded (ARM/RISC-V)
- Toolchain croisé + `linker.ld` fournis dans `examples/kernel/armv7em/`.  
- Sous-ensemble `no_std` : `prelude, string, collections (réduit), random, checksum, rle, mathx, time (monotone)`.  
Build :
```bash
make -C examples/kernel/armv7em
```

### Desktop (GTK/Qt stubs)
- Voir `desktop/` : `gtk_stub.c`, `qt_stub.cpp`, `main.vitte`.  
- Choisir l’impl réelle (gtk_real.c / qt_real.cpp) dans le Makefile.

---

## CI/CD GitHub Actions

`.github/workflows/ci.yml`
```yaml
name: ci
on:
  push:
  pull_request:
jobs:
  build-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install deps
        run: sudo apt-get update && sudo apt-get install -y llvm clang make
      - name: Build
        run: ./scripts/build_all.sh
      - name: Lint & Fmt
        run: ./scripts/ci_check.sh
      - name: Tests
        run: ./scripts/test_all.sh
```

---

## Dépannage (FAQ)

**Q : “`fs` ne marche pas en WASM ?”**  
R : Normal. Utiliser l’API réseau restreinte (fetch-like) + `http_client`.

**Q : “Écritures crash-safe ?”**  
R : `fs_atomic::write_*` (fallback Windows documenté).

**Q : “Imports introuvables ?”**  
R : Vérifie `vitte.toml` `[dependencies]` et le `workspace`.

**Q : “Conflits d’ownership / borrow”**  
R : Utilise `&` (partagé) ou `&mut` (exclusif). Pour partages concurrents : `channel`, `Mutex`.

**Q : “Le binaire est lent en debug”**  
R : `vitc build -p release` et active `--backend llvm` si tu veux du natif.

---

## Bonnes pratiques & perfs

- **Erreurs** : jamais de `panic!` en public API prévue → `Result`.  
- **I/O** : bufferise, préfère `read_to_string` à du byte-par-byte.  
- **Concurrence** : batch + `channel`, évite la contention sur `Mutex`.  
- **Allocations** : `Vec.reserve`, réutilise buffers, `pool` si critique.  
- **Retry** : backoff exponentiel + jitter ; idempotence côté appelant.  
- **Logs** : `info` en prod, `debug`/`trace` contrôlés par env.

---

## Aller plus loin

- **Spécification du langage** : `language-spec.md`  
- **Standard Library** : `stdlib.md`  
- **FFI / ABI** : `ffi.md`  
- **Arborescence** : `docs/arborescence.md`  
- **Contribuer** : `contributing.md`, `code-style.md`, `rfcs/0000-template.md`

---

> _“Code vite. Code sûr. Code Vitte.”_  
Sors des sentiers : profile, bench, ouvre une RFC — et montre-nous ce que ton code vaut.
