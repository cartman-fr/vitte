# Vitte RFCs — `index.md`

> Le cerveau collectif du langage **Vitte**. Ici, chaque idée passe du feu follet au granit : discutée, challengée, gravée. On garde l’esprit frondeur de la Gen Z, mais on respecte la liturgie des bons langages. ✨🛠️

---

## 1) Qu’est-ce qu’une RFC ?
Une **RFC (Request For Comments)** est une proposition formelle d’évolution pour Vitte : syntaxe, sémantique, outillage, standard library, performance, sécurité, process…  
Chaque RFC vit un **cycle** transparent : *DRAFT → DISCUSSION → ACCEPTED/REJECTED → IMPLEMENTED → STABILIZED/DEPRECATED*.  
Ce référentiel est la **source de vérité** du design de Vitte.

---

## 2) Périmètre (Scope)
- **Langage** : grammaire, types, mémoire, erreurs, ABI/FFI, unsafe, const-eval.
- **Toolchain** : `vittec`, `vitpm`, `vitbuild`, `vitfmt`, lints, diagnostics.
- **Stdlib** : modules, API, stabilité, perfs, allocations.
- **Interop** : C, C++, Rust, Wasm, systèmes embarqués.
- **Process** : édition, stabilisation, deprecation, security policy.

---

## 3) Statuts & légende
- 📝 `DRAFT` — brouillon ouvert aux retours.
- 💬 `DISCUSSION` — débat structuré (issues/PR/meeting).
- ✅ `ACCEPTED` — design approuvé, prêt à implémenter.
- ❌ `REJECTED` — refusé (garde des notes pour l’historique).
- 🚧 `IN PROGRESS` — implémentation en cours.
- 🧪 `EXPERIMENTAL` — feature guardée derrière un flag.
- 📦 `IMPLEMENTED` — code mergé, docs et tests présents.
- 🪵 `STABILIZED` — API gelée selon la politique de stabilité.
- 🕯️ `DEPRECATED` — en voie de retrait.
- 🗿 `OBSOLETE` — remplacé par une autre RFC.

---

## 4) Numérotation & nommage
- Format : `NNNN-kebab-case-titre.md` (ex. `0006-async-await-concurrency.md`)
- Numérotation **séquentielle** lors de l’ouverture du PR (pas d’édition rétroactive).
- Un fichier = **un sujet autonome**.

---

## 5) Politique de version (SemVer + Éditions)
- **SemVer** : `MAJOR.MINOR.PATCH` pour la toolchain & stdlib.  
- **Éditions** du langage (à la Rust) : grands jalons de sémantique/syntaxe.  
- Ruptures : seulement via **Édition** ou derrière un **feature flag** longuement incubé.  
- Stabilité doc : APIs marquées `@stable`, `@unstable(feature="…")`.

---

## 6) Cycle de vie d’une RFC

Authoring ──> DRAFT ──> DISCUSSION ──┬──> REJECTED
└──> ACCEPTED ──> IN PROGRESS ──> IMPLEMENTED
└──> EXPERIMENTAL (flag)
IMPLEMENTED ──> STABILIZED ──> (éventuelle) DEPRECATED ──> OBSOLETE


**Entrées/sorties obligatoires :**
- **DRAFT → DISCUSSION** : motivation claire, alternatives, protos, benchmarks si perfs.
- **DISCUSSION → ACCEPTED** : consensus mainteneurs + décision écrite.
- **ACCEPTED → IMPLEMENTED** : PRs liées, plan de migration, tests.
- **STABILIZED** : doc finalisée, lints & diagnostics prêts, compat prouvée.

---

## 7) Rôles & gouvernance
- **Auteur·e(s)** : rédigent, répondent aux retours, maintiennent la RFC.
- **Relecteurs** (langage, VM, stdlib, toolchain, secu) : exigent, bienveillants.
- **Mainteneurs** : arbitrent, valident l’acceptation, garantissent la cohérence.
- **WG (Working Groups)** : thèmes (perf, async, FFI, ergonomie, safety…).

---

## 8) Comment proposer une RFC (workflow express)
1. **Fork + branche** : `rfcs/NNNN-titre-court`.
2. **Copier le gabarit** : [`TEMPLATE.md`](./TEMPLATE.md) → remplir toutes les sections.
3. **Fichier** : `rfcs/NNNN-titre.md` + **`motivation/`, `prototypes/`, `benches/`** si utiles.
4. **PR** : lien vers discussion/issue, sommaire clair, impact, plan de migration.
5. **Revue** : itérations rapides, décisions documentées dans la RFC (section *Rationale*).
6. **Décision** : `ACCEPTED` ou `REJECTED` + justification concise.
7. **Suivi** : créer/relier les issues d’implémentation & docs.

**Règle d’or** : *Pas d’implémentation mergée sans RFC ACCEPTED pour les features non triviales.*

---

## 9) Compatibilité & migration
- Toute rupture **explique** : pourquoi, alternatives, guide de migration, lints automatiques.
- Outils : `vitfix` (codemods), lints « future-incompatible », messages DX béton.
- Dépréciation **graduelle** : warnings → gated → retrait à la prochaine Édition.

---

## 10) Sécurité
- Section **Security Considerations** obligatoire pour : unsafe, FFI, UB, sandbox, crypto.
- Révélations privées → security@vitte.dev (PGP).  
- Bench & fuzz si la surface est sensible (allocateurs, JIT, parser).

---

## 11) Style & preuves
- **Exemples compilables** (doctests ou `examples/`).
- **Pseudocode → code** : viser un prototype minimal reproductible.
- **Benchmarks** : préciser CPU/OS/flags, `criterion` ou équivalent, comparer N variantes.
- **Terminologie** : définir les termes, éviter l’ambiguïté.

---

## 12) Réunions & cadence
- **Design sync** : bimensuel, notes publiques, décisions tracées ici.
- **Fast-track** (trivial) : petites clarifications de docs, orthographe, lints évidents.

---

## 13) Index des RFCs (vivant)

> Tri : par numéro croissant. *Last-Updated* = date de la dernière modification de la RFC.

| #     | Titre | Statut | Domaine | Auteur(s) | Last-Updated | Lien |
|------:|-------|:------:|---------|-----------|:-------------|------|
| 0001 | Core syntax & keywords | 💬 DISCUSSION | Langage | @core-team | 2025-08-15 | [0001-core-syntax-and-keywords.md](./0001-core-syntax-and-keywords.md) |
| 0002 | Module system & visibility | 📝 DRAFT | Langage | @modules-wg | 2025-08-15 | [0002-module-system.md](./0002-module-system.md) |
| 0003 | Memory model & ownership | 💬 DISCUSSION | Langage/VM | @safety-wg | 2025-08-15 | [0003-memory-model-and-ownership.md](./0003-memory-model-and-ownership.md) |
| 0004 | Error handling (Result, panics, lints) | 📝 DRAFT | Langage/DX | @dx-wg | 2025-08-15 | [0004-error-handling.md](./0004-error-handling.md) |
| 0005 | FFI & Interop (C/Rust/Wasm) | 📝 DRAFT | Interop | @ffi-wg | 2025-08-15 | [0005-ffi-and-interoperability.md](./0005-ffi-and-interoperability.md) |
| 0006 | Async/await & scheduler | 💬 DISCUSSION | Concurrency | @async-wg | 2025-08-15 | [0006-async-await-concurrency.md](./0006-async-await-concurrency.md) |
| 0007 | Pattern matching & exhaustivité | 📝 DRAFT | Langage | @lang-wg | 2025-08-15 | [0007-pattern-matching.md](./0007-pattern-matching.md) |
| 0008 | Macro system (hygiénique, compile-time) | 📝 DRAFT | Langage | @macro-wg | 2025-08-15 | [0008-macro-system.md](./0008-macro-system.md) |
| 0009 | Stdlib layout & stability | 💬 DISCUSSION | Stdlib | @stdlib-wg | 2025-08-15 | [0009-std-library-structure.md](./0009-std-library-structure.md) |
| 0010 | Package manager `vitpm` | 📝 DRAFT | Toolchain | @tooling-wg | 2025-08-15 | [0010-package-manager-vitpm.md](./0010-package-manager-vitpm.md) |
| 0011 | Build system `vitbuild` | 📝 DRAFT | Toolchain | @build-wg | 2025-08-15 | [0011-build-system-vitbuild.md](./0011-build-system-vitbuild.md) |
| 0012 | Formatting `vitfmt` + lints | 📝 DRAFT | Tooling/DX | @fmt-wg | 2025-08-15 | [0012-formatting-and-lints.md](./0012-formatting-and-lints.md) |
| 0013 | Const-eval & compile-time | 📝 DRAFT | Langage | @ct-wg | 2025-08-15 | [0013-const-eval.md](./0013-const-eval.md) |
| 0014 | Unsafe guidelines & UB | 💬 DISCUSSION | Safety | @safety-wg | 2025-08-15 | [0014-unsafe-guidelines.md](./0014-unsafe-guidelines.md) |
| 0015 | Diagnostics & error codes | 📝 DRAFT | DX/Toolchain | @dx-wg | 2025-08-15 | [0015-diagnostics-and-codes.md](./0015-diagnostics-and-codes.md) |
| 0016 | Wasm target & sandbox | 📝 DRAFT | Backend | @wasm-wg | 2025-08-15 | [0016-wasm-target.md](./0016-wasm-target.md) |
| 0017 | Embedded & `no_std` | 📝 DRAFT | Embedded | @embedded-wg | 2025-08-15 | [0017-embedded-no-std.md](./0017-embedded-no-std.md) |
| 0018 | ABI & calling conv. | 💬 DISCUSSION | Interop | @abi-wg | 2025-08-15 | [0018-abi-calling-conventions.md](./0018-abi-calling-conventions.md) |
| 0019 | Iterators & async iter | 📝 DRAFT | Langage | @lang-wg | 2025-08-15 | [0019-iterators.md](./0019-iterators.md) |
| 0020 | Modules crypto (draft) | 📝 DRAFT | Stdlib | @crypto-wg | 2025-08-15 | [0020-stdlib-crypto.md](./0020-stdlib-crypto.md) |

> Ajoutez les nouvelles entrées ici, gardez la table **triée par #**, mettez `Last-Updated` à jour à chaque commit.

---

## 14) Liens utiles
- **Gabarit** : [`TEMPLATE.md`](./TEMPLATE.md)  
- **Guide de contribution** : `../CONTRIBUTING.md`  
- **Code of Conduct** : `../CODE_OF_CONDUCT.md`  
- **Process de release & éditions** : `../docs/release-and-editions.md`  
- **Security policy** : `../SECURITY.md`  
- **Changelog agrégé des RFCs** : `./CHANGELOG.md`

---

## 15) FAQ (cash & sans chichi)
**Q : Puis-je ouvrir une RFC “idée floue” ?**  
R : Oui, mais assumez le *DRAFT* et venez avec cas d’usage + alternatives. On ne shippe pas des vibes.

**Q : Et si je ne suis pas d’accord avec la décision ?**  
R : Proposez des **benchmarks**, un **prototype**, ou un **counter-design** concret. Les faits gagnent.

**Q : Faut-il implémenter avant d’accepter ?**  
R : Non, mais une *proof of concept* accélère tout. On aime le réel.

**Q : Combien de temps dure la discussion ?**  
R : Tant qu’il faut pour converger, pas un jour de plus. Les mainteneurs tranchent.

---

## 16) Philosophie (boussole)
- **Rapide, sûr, sobre** : perfs, safety, et ergonomie de haut vol, sans folklore.
- **Clarté > magie** : des règles nettes, des diagnostics pédagogiques.
- **Mesurable** : chaque promesse se paie en benchmarks et en tests.
- **Respect du passé** : interop clean, migration guidée, zéro mépris pour l’existant.

> *Bref : on rêve haut, on code serré, on documente propre. En avant, Vitte.* 🚀
