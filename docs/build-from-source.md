# Code Style — **Vitte** (code-style.md • ultra-complet)

> _“Lisible aujourd’hui, maintenable demain, performant toujours.”_  
Ce guide définit le **style officiel** pour : **Vitte** (`.vitte/.vit`), **Rust** (crates/outillage), **C/C++** (stubs desktop/embedded), **Shell** (scripts), et **Docs** (Markdown).  
Le formatage **automatique** est **obligatoire** : `vitte-fmt` pour Vitte, `rustfmt` pour Rust.

---

## 0) Principes non-négociables

1. **Lisibilité > concision magique** : code explicite, coûts visibles (alloc, I/O, verrous).  
2. **Sûreté** : pas de `panic!` en API publique pour erreurs attendues → **`Result`**.  
3. **Prévisibilité** : conventions de nommage stables, pas d’abus de sur-généricité.  
4. **Diffs propres** : trailing commas, imports triés, formatage auto.  
5. **Docs + tests** : chaque API publique a une doc et au moins un test.

---

## 1) Formatage — Vitte

- **Indentation** : 2 espaces.  
- **Largeur** : 100 colonnes max.  
- **Accolades** : K&R (`do f(){ ... }`) ; accolade ouvrante **collée**.  
- **Espaces** : `if cond {` • `a + b` • `fn(x: T, y: U)`.  
- **Virgules terminales** en multi-ligne.  
- **Imports** triés : standard `std::*`, puis `modules::*`, puis locaux (`use crate::*`), par ordre lexicographique.  
- **Blocs vides** sur une ligne : `do f() {}` (rare, commente la raison).  
- **Fichiers** terminés par `\n`.

### 1.1 Exemple

```vitte
use std::string
use std::time
use modules::retry

struct Config { endpoint: String, timeout_ms: u32 }

do request(cfg &: Config) -> Result[String, str] {
  let p = retry::exponential_backoff(
    max_retries: 5,
    base_ms: 100,
    jitter: true,
  )
  match retry::run(p, || http_client::get(cfg.endpoint)) {
    Ok(r)  => Ok(string::from_bytes(r.body)),
    Err(e) => Err(to_string(e)),
  }
}
```

---

## 2) Nommage — Vitte

- **Modules & fichiers** : `snake_case` (`web_echo`, `yaml_lite`).  
- **Types/Enums/Traits** : `CamelCase` (`HttpError`, `Display`).  
- **Constantes** : `SCREAMING_SNAKE_CASE`.  
- **Fonctions/vars** : `snake_case`.  
- **Génériques** : `T, U, E` (sigles courts) ; spécifiques : `TItem`, `TKey`.  
- **Suffixes** :  
  - Async : `*_async`  
  - Non bloquant : `try_*`  
  - Invariants forts : `*_strict`  
  - Conversions : `to_*`, `from_*`  
- **Erreurs** : suffixer `*Error`.

> Éviter les bools nus en paramètres ; préférer un **enum** à deux valeurs (`Mode::Fast | Safe`).

---

## 3) Organisation d’un fichier — Vitte

Ordre recommandé :
1. `module` + **doc de module** (`//!` en tête — *optionnel si supporté*)  
2. `use` groupés/triés  
3. `pub` types/enums/consts  
4. impl/traits publics  
5. items internes  
6. tests (en bas, balisés `@test` si dispo)

---

## 4) Fonctions & API

- **API publiques** : jamais de `panic!` pour erreurs attendues → `Result[T,E]`.  
- **Entrées** : préférer **slices** (`[]T`, `str`) et **références** (`&T`) aux copies.  
- **Sorties** : préférer `Result` clair ; pas de codes magiques.  
- **Paramètres > 4** : utiliser un **struct**‐param.  
- **Effets** : pas d’effets cachés dans des getters ; `get_*` est **sans effets**.

```vitte
do normalize(input: []u8, opts &: Options) -> Result[String, NormError]
```

---

## 5) Structs, Enums, Traits

- **Structs** : champs publics **seulement si** invariants solides ; sinon `pub` méthodes.  
- **Constructeurs** : `new`, `with_*`, builder si nécessaire.  
- **Enums** : variants **nommés**, match **exhaustif**.  
- **Traits** : noms **adjectifs** ou **capacité** (`Display`, `Serialize`). Fournir des **défauts** sûrs.

---

## 6) Ownership & Borrowing

- Passer les **gros objets** par `&` (immuable) / `&mut` (exclusif).  
- **Pas de clone facile** : évite `clone()` par défaut. Préfère vues/slices.  
- Durées de vie **inférées** ; si ambigu, factoriser le code.

---

## 7) Concurrence

- **Canaux** (`channel`) pour la comm ; **éviter** les verrous larges.  
- **Ne pas** tenir un verrou à travers un `await` (quand async stable).  
- Documenter `Send`/`Sync` conceptuel : “type X est thread-safe pour lecture, pas écriture”.

---

## 8) I/O, temps et retry

- **Écritures critiques** : `fs_atomic::*`.  
- **HTTP** : timeouts explicites ; replays **idempotents** seulement.  
- **Retry** : backoff exponentiel + jitter ; **doc** sur l’idempotence.

---

## 9) Erreurs & messages

- Messages concis, **actionnables** :  
  ```
  "config: key 'endpoint' missing (file: app.ini)"
  ```
- Pas d’info‐leak : ne jamais révéler chemins secrets / clés.  
- Chaînage d’erreurs : ajoute le **contexte** près de la source.

---

## 10) Docs (Vitte)

- **Doc de module** en tête (`//!` si supporté).  
- **Doc d’API** (`///`) avec : but, préconditions, erreurs, complexité (si utile), **exemple compilable**.  
- Blocs de code annotés `vitte`.  
- Référencer les types avec leurs **chemins** (`std::fs::Path`).

Exemple :

```vitte
/// Calcule la moyenne arithmétique.
/// 
/// # Erreurs
/// - `Empty`: si la slice est vide.
/// 
/// # Exemple
/// ```vitte
/// assert(mean([1,2,3]).unwrap()==2)
/// ```
do mean(xs: []i64) -> Result[i64, EmptyError] { ... }
```

---

## 11) Tests

- Nommer `feature_scenario_expected`.  
- **Happy path** + cas d’erreurs.  
- Pas de dépendance réseau en unit ; mock/fake.  
- Tests déterministes : random **seedé**.  
- Performance : microbench séparés des unit tests.

---

## 12) Style — **Rust** (crates/outils)

- `rustfmt` + `clippy` + `deny.toml` **obligatoires**.  
- `#![deny(warnings)]` dans crates critiques (compiler/vm/runtime).  
- `unsafe` : commentaire `// SAFETY: raison, invariants, preuves`.  
- Erreurs avec `thiserror`-like ; `anyhow` toléré **uniquement outils**.  
- Pas de `unwrap()/expect()` en prod (ok en tests/outils).  
- Modules : `mod.rs` évité ; fichiers explicites.

**Imports Rust** : `std`, **puis** extern crates, **puis** local. Triés, sans wildcard sauf tests.

---

## 13) Style — **C/C++** (stubs desktop/embedded)

- C99/C11 ; C++17 si requis (Qt).  
- **Headers** gardés (`#pragma once`).  
- **Types fixes** (`stdint.h`).  
- Pointeurs **const** quand possible ; **no** `void*` sans doc.  
- ABI : fonctions `extern "C"` ; **ptr+len** pour buffers.  
- `// SAFETY:` sur sections sensibles.  
- Formatage : 2 espaces, 100 colonnes, K&R.

---

## 14) Style — **Shell** (scripts)

- `bash -euo pipefail` en tête.  
- Fonctions, `trap` pour cleanup.  
- Variables en MAJUSCULE pour env, `readonly` si possible.  
- Pas de `sudo` implicite dans les scripts (documente).

---

## 15) Lints & config

### 15.1 vitte-fmt (exemple)
```toml
# vitte-fmt.toml
max_width = 100
indent_spaces = 2
trailing_commas = true
space_around_operators = true
newline_at_eof = true
```

### 15.2 rustfmt/clippy
```toml
# rustfmt.toml
max_width = 100
use_small_heuristics = "Max"
newline_style = "Unix"

# clippy.toml
warns = ["clippy::pedantic"]
allow-unwrap-in-tests = true
```

### 15.3 deny (Rust)
- **deny** : `warnings`, `unsafe_op_in_unsafe_fn`, `unreachable_pub`, `unused_results` (où pertinent).  
- **allow** limité et commenté.

---

## 16) Performance & mémoire

- Éviter les **allocations** répétées (pré-allouer, réutiliser buffers).  
- Préférer slices aux copies.  
- **Pas** de dispatch dynamique dans les hot-loops si évitable.  
- Mesurer : `vitte-bench` + profiler. **Pas d’intuition non mesurée**.

---

## 17) Sécurité

- Entrées non fiables → **valider** (`validate`), tailles/limites strictes.  
- Fichiers : pas d’**overwrite** surprise ; préférer `fs_atomic`.  
- Réseau : **timeouts** obligatoires, `tls` activé.  
- Secrets : **jamais** dans les logs.  
- FFI : `extern(c)`, **ptr+len**, qui alloue libère. Panics **contenus**.

---

## 18) Exemples “bon/mauvais”

### 18.1 Erreur publique

**✅ Bon**
```vitte
do read_cfg(path: str) -> Result[String, CfgError] {
  fs::read_to_string(path).map_err(|e| CfgError::Io(to_string(e)))
}
```

**❌ Mauvais**
```vitte
do read_cfg(path: str) -> String {
  // panic en cas d’échec → interdit
  fs::read_to_string(path).unwrap()
}
```

### 18.2 Param bool flou

**✅ Bon**
```vitte
enum Mode { Safe, Fast }
do compute(xs: []u8, mode: Mode) { ... }
```

**❌ Mauvais**
```vitte
do compute(xs: []u8, fast: bool) { ... }
```

### 18.3 Lock + async

**✅ Bon**
```vitte
let data = { let g = mtx.lock(); clone(*g) } // relâche le lock
await net::send(data)
```

**❌ Mauvais**
```vitte
let g = mtx.lock()
await net::send(*g) // tient le lock à travers await → risque d’interblocage
```

---

## 19) En-têtes & bannières

- Pas d’entêtes de licence verbeux par fichier (licence à la racine).  
- En-tête optionnel minimal :
```vitte
//! Module scheduler — tâches différées/périodiques.
//! Invariants: jamais de tâche zombie ; horloge monotone.
```

---

## 20) Git & diffs propres

- Toujours **formater** avant commit (`vitte-fmt`, `rustfmt`).  
- Commits **impératifs** et **scopés** (`feat(stdlib): add uuid v7`).  
- Pas de fichiers générés en repo (sauf exceptions documentées).

---

## 21) Glossaire rapide

- **Stable** : API gelée (patch/minor safe).  
- **Preview** : sous révision, peut bouger.  
- **Experimental** : aucune garantie, peut casser.  
- **Hot path** : section sensible aux perfs.  
- **Handle** : pointeur opaque côté FFI.

---

## 22) Checklist PR style

- [ ] `vitte-fmt`/`rustfmt` pass  
- [ ] Imports triés, pas de dead code public  
- [ ] Doc `///` + exemple  
- [ ] Tests (happy + erreurs)  
- [ ] Pas de `panic!` en public API  
- [ ] Lints/deny OK  
- [ ] Changelog/doc mis à jour si surface publique

---

**Mantra** : _“Fais simple, montre tes coûts, écris la doc, mesure les perfs.”_  
Le style n’est pas une option : c’est notre contrat de lecture.
