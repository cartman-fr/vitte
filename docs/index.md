# Vitte Language — Documentation Portal (index.md • **ultra-complet**)

> **Vitte** : un langage moderne qui marie la clarté, la sûreté et la vitesse.  
> **Édition** : 2025 • **Toolchain minimal** : ≥ 0.6.0 • **Statut** : stable + zones *preview/experimental* clairement balisées.

---

## 🧭 Liens rapides

- 📦 **Téléchargements** : binaires & source (voir *Build from Source* ci-dessous)  
- 🚀 **Getting Started** → [`getting-started.md`](getting-started.md)  
- 📜 **Language Spec** → [`language-spec.md`](language-spec.md)  
- 🧰 **Standard Library** → [`stdlib.md`](stdlib.md)  
- 🔗 **FFI / ABI C** → [`ffi.md`](ffi.md)  
- 🔨 **Build from Source** → [`build-from-source.md`](build-from-source.md)  
- 🤝 **Contribuer** → [`contributing.md`](contributing.md) • 🎯 **Style** → [`code-style.md`](code-style.md)  
- 🗺️ **Arborescence canonique** → `../arborescence.md` • `docs/arborescence.md`  
- 🧪 **Tests/Benchs** → `../scripts/test_all.sh` • `../scripts/gen_docs.sh`  
- 🧷 **Sécurité** → `../security/` • 👩‍⚖️ **Licences** → `../LICENSE`

---

## 📚 Table des matières

1. [Pourquoi Vitte ?](#pourquoi-vitte)
2. [Installation](#installation)
   - [A. Binaire précompilé](#a-binaire-précompilé)
   - [B. Via Rust/Cargo](#b-via-rustcargo)
   - [C. Depuis le code source](#c-depuis-le-code-source)
3. [Hello, Vitte ! (exemples)](#hello-vitte-exemples)
4. [Structure d’un projet](#structure-dun-projet)
5. [Cibles & Backends](#cibles--backends)
6. [Outils du toolchain](#outils-du-toolchain)
7. [Flux de dev : build, test, bench, docs](#flux-de-dev--build-test-bench-docs)
8. [Versionnage, stabilité & éditions](#versionnage-stabilité--éditions)
9. [FAQ & dépannage](#faq--dépannage)
10. [Contribuer, gouvernance & code de conduite](#contribuer-gouvernance--code-de-conduite)
11. [Feuille de route & RFC](#feuille-de-route--rfc)
12. [Crédits & licences](#crédits--licences)

---

## Pourquoi Vitte ?

- **Sûreté** : API `Result/Option`, emprunts (`&` / `&mut`), panics réservés aux invariants.  
- **Perfs natives** : VM bytecode **et** backends natifs (LLVM/JIT).  
- **Interop claire** : FFI C sans magie, ABI documentée, buffers explicites.  
- **Polyvalent** : desktop, serveurs, embarqué, WebAssembly, kernel.  
- **Ergonomie** : `do`, `match`, `async` (preview), modules lisibles, outils intégrés.

👉 Détails dans [`language-spec.md`](language-spec.md) et la stdlib dans [`stdlib.md`](stdlib.md).

---

## Installation

### A. Binaire précompilé
Téléchargez l’archive correspondant à votre OS/arch puis placez le binaire dans votre `PATH` :
```bash
# Exemple Linux x86_64
tar -xzf vitte-linux-x86_64.tar.gz
sudo mv vitte /usr/local/bin/
vitte --version
```

### B. Via Rust/Cargo
Le projet fournit des crates CLI (voir `crates/vitte-cli`) :
```bash
# Prérequis : Rust stable + LLVM (pour backend natif)
rustup toolchain install stable
cargo install --path ./crates/vitte-cli
vitte --help
```

### C. Depuis le code source
Guide complet : [`build-from-source.md`](build-from-source.md)  
Raccourci :
```bash
# racine du repo
./scripts/ci_check.sh      # lints + fmt + build rapide
./scripts/build_all.sh     # build complet (VM + LLVM + tools)
./scripts/test_all.sh      # exécution test suite
```

---

## Hello, Vitte ! (exemples)

Fichier `examples/hello/src/main.vit` :
```vitte
do main() { print("Hello, Vitte!") }
```

Compiler et exécuter :
```bash
# avec vitc / vitte-cli (selon votre alias)
vitc build examples/hello
vitc run examples/hello
```

HTTP + retry (extrait doc stdlib) :
```vitte
use http_client; use retry; use string

do fetch_with_retry(url: str) -> Result[String, str] {
  let p = retry::exponential_backoff(max_retries: 4, base_ms: 80, jitter: true)
  match retry::run(p, || http_client::get(url)) {
    Ok(r)  => Ok(string::from_bytes(r.body)),
    Err(e) => Err(to_string(e)),
  }
}
```

---

## Structure d’un projet

- **Monorepo Vitte** : `crates/` (compiler, runtime, VM, stdlib, outils), `modules/` (kits additionnels), `examples/`, `docs/`.  
- **Projet app** (mini) :
```
my-app/
├── vitte.toml
└── src/
   └── main.vit
```
Modèle conseillé et variations → [`getting-started.md`](getting-started.md) + `docs/arborescence.md`.

---

## Cibles & Backends

| Plateforme | VM (bytecode) | LLVM (native) | Cranelift (JIT) |
|---:|:---:|:---:|:---:|
| Linux x86_64 | ✅ | ✅ | ✅ |
| macOS (Intel/Apple) | ✅ | ✅ | ✅ |
| Windows x64 | ✅ | ✅ | ✅ |
| BSD | ✅ | ✅ | ✅ |
| WASM | ⚠️ partiel | n/a | n/a |
| Embedded (ARM/RISC-V) | ✅ | ✅ | n/a |

> WASM : `fs/process` indisponibles ; réseau restreint. Embarqué : sous-ensemble `no_std` (voir [`stdlib.md`](stdlib.md)).

---

## Outils du toolchain

| Outil | Rôle | Dossier |
|---|---|---|
| `vitc` | Compiler (frontend + backends) | `tools/vitc` |
| `vitcc` | Variantes/expérimentations du compilo | `tools/vitcc` |
| `vitpm` | Gestionnaire de paquets/projets | `tools/vitpm` |
| `vitte-fmt` | Formateur de code | `tools/vitte-fmt` |
| `vitte-bench` | Suite de benchmark | `tools/vitte-bench` |
| `vitte-profile` | Profiler | `tools/vitte-profile` |
| `vitte-doc` | Génération de docs | `tools/vitte-doc` |
| `vitte-asm/disasm/link` | Outils bytecode/IR | `crates/vitte-tools` |

> Astuce VS Code : config `tasks.json` pour binder `vitc build`, `vitte-fmt`, et `test`.

---

## Flux de dev : build, test, bench, docs

```bash
# Formatage + lints
./scripts/fmt.sh
./scripts/lint.sh

# Build complet (tous crates + outils)
./scripts/build_all.sh

# Tests (unit + integration + vm + perf)
./scripts/test_all.sh
./scripts/ci_check.sh

# Benchmarks
./scripts/gen_bytecode.sh   # génère/rafraîchit les opcodes
cargo bench -p vitte-bench  # (ou via ./tools/vitte-bench)

# Docs (GitHub Pages / portail docs/)
./scripts/gen_docs.sh
```

---

## Versionnage, stabilité & éditions

- **Niveaux** : `stable` (gel rétro-compat), `preview` (retours), `experimental` (peut casser).  
- **Éditions** : ruptures regroupées (ex : 2025, 2026) → migration guidée.  
- **SemVer** : `MAJOR.MINOR.PATCH` sur l’outillage et les crates publiques.

Voir le détail dans : [`language-spec.md`](language-spec.md) & [`contributing.md`](contributing.md).

---

## FAQ & dépannage

**Q. “`fs` ne marche pas en WASM ?”**  
R. Normal. Utilisez l’API restreinte (fetch-like) ; privilégiez `http_client` + `string`.

**Q. “Je veux des écritures fichiers crash-safe.”**  
R. Utilisez `fs_atomic::write_*` (fallback Windows documenté).

**Q. “Comment générer l’arborescence Markdown depuis VS Code ?”**  
R. Minimal :  
```bash
# depuis la racine du repo
git ls-tree -r --name-only HEAD | sed 's|^|- |' > docs/tree.md
```
Ou script Node/TS pour un rendu “fancy” avec tailles/ignores (voir `docs/arborescence.md` pour un snippet prêt-à-l’emploi).

**Q. “Je veux packager des modules custom.”**  
R. Voir `modules/README.md` + `tools/vitpm` (schéma de manifest, hooks `post-install`).

---

## Contribuer, gouvernance & code de conduite

- **PR flow** : issues → RFC (si langage/ABI) → PR petite et testée.  
- **Commits** : conventionnels (`feat:`, `fix:`, `docs:`, `refactor:`, `perf:`…).  
- **Tests obligatoires** : unitaires + intégration + perf si régression sensible.  
- **Gouvernance** : `CODEOWNERS`, review 2 pairs pour crates critiques.  
- **Sécurité** : signalements privés dans `../security/` (processus détaillé).  

Guides : [`contributing.md`](contributing.md) • [`code-style.md`](code-style.md) • `rfcs/0000-template.md`.

---

## Feuille de route & RFC

- **Roadmap** (extraits) :
  - Async/await “édition 2026” (stabilisation du runtime + I/O non bloquante).
  - WASM “fetch-first” + FS virtuel.
  - `uuid` v7 stable, `idgen` distribué.
  - Améliorations FFI : callbacks, layout `extern(c)` struct stable.
- **RFCs** : `rfcs/` (index + discussions). Proposez, benchmarquez, documentez.

---

## Crédits & licences

- **Auteurs & contributors** : voir l’historique Git & `CODEOWNERS`.  
- **Licence** : MIT **ou** Apache-2.0 (cf. `../LICENSE`).  
- **Marques** : “Vitte” est un projet open-source ; respectez les guidelines de nommage de forks/plugins.

---

## Annexes utiles

- **Exemples** : `examples/hello`, `examples/web-echo`, `examples/wasm-add`, `examples/kernel/*`, `examples/worker-jobs`.  
- **Modules clés** : `modules/` (log, config, http_client, kvstore, scheduler, retry, rate_limiter, ...).  
- **Outils avancés** : `tools/vitx`, `tools/vitxx` (expérimentations).  
- **Docs internes** : `docs/stdlib.md`, `docs/language-spec.md`, `docs/ffi.md`, `docs/arborescence.md`.

---

> _“Codez vite. Codez sûr. Codez Vitte.”_  
> Pour tout le reste, plongez dans le code et laissez parler les benchmarks.
