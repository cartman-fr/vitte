# Contributing to **Vitte** (contributing.md — ultra complet)

> _“Un langage se bâtit comme une cathédrale : pierre après pierre, test après test.”_  
> Merci de vouloir contribuer à **Vitte**. Ce guide te donne tout — des règles de style au process de release — pour des PRs nettes et mergeables.

---

## 🧭 Sommaire

1. [Philosophie & principes](#philosophie--principes)  
2. [Panorama du monorepo](#panorama-du-monorepo)  
3. [Pré-requis & setup](#pré-requis--setup)  
4. [Issues & triage](#issues--triage)  
5. [Branches, commits, PRs](#branches-commits-prs)  
6. [Style & lints (Vitte + Rust + Docs)](#style--lints-vitte--rust--docs)  
7. [Tests, CI & qualité](#tests-ci--qualité)  
8. [Benchmarks & perfs](#benchmarks--perfs)  
9. [Changement de langage (RFCs)](#changement-de-langage-rfcs)  
10. [Stabilité, SemVer & Éditions](#stabilité-semver--éditions)  
11. [Process de dépréciation](#process-de-dépréciation)  
12. [Sécurité & signalements](#sécurité--signalements)  
13. [Plugins, FFI & ABI](#plugins-ffi--abi)  
14. [Docs & site](#docs--site)  
15. [Checklists par domaine](#checklists-par-domaine)  
16. [Code de conduite](#code-de-conduite)  
17. [Licence & Droit](#licence--droit)  
18. [FAQ contributeur](#faq-contributeur)

---

## Philosophie & principes

- **Sûreté par défaut** : `Result/Option` en API publique, pas de `panic!` hors invariants.  
- **Coût explicite** : alloc/I/O/verrous visibles dans les signatures.  
- **Stabilité graduelle** : `stable` / `preview` / `experimental`, promues par RFC + gel.  
- **Qualité mesurable** : CI verte, test coverage pertinent, benchmarks reproductibles.  
- **Concision, pas complexité** : une PR = une idée. Petites étapes, gros impact.

---

## Panorama du monorepo

```
.
├── crates/          # vitte-cli, vitte-compiler, vitte-core, vitte-runtime, vitte-vm, vitte-tools, stdlib…
├── modules/         # log, config, http_client, kvstore, scheduler, retry, …
├── tools/           # vitc, vitcc, vitpm, vitte-fmt, vitte-bench, vitte-doc, vitte-profile, vitx, vitxx
├── examples/        # hello, web-echo, wasm-add, kernel/*, worker-jobs…
├── docs/            # index.md, language-spec.md, stdlib.md, getting-started.md, ffi.md…
├── scripts/         # build_all.sh, test_all.sh, ci_check.sh, gen_docs.sh, gen_bytecode.sh…
├── .github/         # workflows/ci.yml, release.yml, templates PR/Issues, CODEOWNERS
└── rfcs/            # 0000-template.md + propositions
```

- **Cœur Rust** : compilo/vm/runtime/outils (dans `crates/` et `tools/`).  
- **Langage Vitte** : fichiers `.vitte/.vit` dans `examples/` & `modules/`.  
- **Docs** : Markdown orienté “portail dev”.

---

## Pré-requis & setup

- **OS** : Linux, macOS (Intel/Apple), Windows x64, BSD.  
- **Outils** : Git ≥ 2.40, LLVM/Clang (backend natif), Make/CMake (selon outils), Python 3.x (scripts), Rust stable via `rustup`.  
- **Éditeur** : VS Code + extension Vitte (syntaxe) si dispo (`editor-plugins/vscode`).  
- **Bootstrap** :
```bash
git clone https://github.com/<org>/vitte.git
cd vitte
./scripts/ci_check.sh      # lint + fmt + build rapide
./scripts/build_all.sh     # build complet (VM + LLVM + tools)
./scripts/test_all.sh      # test suite
```

---

## Issues & triage

- **Types** : `bug`, `feat`, `perf`, `docs`, `refactor`, `infra`, `question`.  
- **Labels** : `good first issue`, `help wanted`, `area/compiler`, `area/vm`, `area/stdlib`, `prio/high`, `blocked`.  
- **SLA triage** : 7 jours ouvrés pour un premier retour.  
- **Repro minimal** requis pour les bugs (petit programme `.vitte` + logs `VITTE_LOG=trace` si pertinent).  
- **Discussion avant dev** pour tout ce qui touche : langage, ABI/FFI, VM, runtime.

---

## Branches, commits, PRs

### Branches
- `main` : stable (protégée).
- `release/*` : branches de sortie.
- `feat/<slug-court>` / `fix/<slug>` / `perf/<slug>`.

### Commits (Conventional Commits)
```
feat(stdlib): add uuid v7 (time-ordered)
fix(vm): correct stack overflow check in call frame
perf(compiler): memoize type resolution in hot path
docs(ffi): clarify ptr+len ownership contract
refactor(runtime): split scheduler into submodules
test(channel): add stress tests for MPMC
```
- Scope = dossier/zone. Messages **impératifs** et **concrets**.  
- **Sign-off (DCO)** recommandé : `git commit -s`.

### Pull Requests
- **Petite** et **focalisée**.  
- Lien vers l’issue (ou RFC).  
- Checklist (voir plus bas) dans la description.  
- CI **verte** (obligatoire).  
- **Reviews** : 1 review suffisante pour docs/outils, **2 reviewers** pour `compiler`, `vm`, `runtime`, `stdlib`.  
- Merge : **squash** par défaut (historique propre).  
- Pas de PNG généré sans source (SVG/mermaid privilégiés).

---

## Style & lints (Vitte + Rust + Docs)

### Vitte (fichiers `.vitte/.vit`)
- **Indentation** : 2 espaces • **Ligne** max 100 colonnes • Virgules terminales en multi-ligne.  
- Nommage : `snake_case` pour fonctions/variables, `CamelCase` pour types, modules concis.  
- **Pas** d’API publique qui `panic!` pour erreur attendue → `Result`.  
- **Sync** vs **async** : suffixer `*_async` (preview).  
- `try_*` = non-bloquant, `*_strict` = invariants forts.

### Rust (crates)
- `rustfmt.toml` + `clippy.toml` + `deny.toml` respectés.  
- `#![deny(warnings)]` sur crates critiques.  
- `unsafe` : **justifier** avec un commentaire `// SAFETY:`.  
- Zéro `unwrap()` en prod code (tests ok).  
- Erreurs chainées (`thiserror`/`anyhow`-like s’il y en a).

### Docs
- Markdown propre, titres hiérarchisés, exemples **compilables** si possible.  
- Tableaux pour matrices de plateformes, blocs de code annotés.  
- Génère avec `./scripts/gen_docs.sh` avant PR.

---

## Tests, CI & qualité

- **Où** :  
  - Rust : `crates/*/src` + `crates/*/tests` (unit/integration).  
  - Vitte : `tests/{unit,vm,compiler,integration,performance}`.  
- **Nommer** les tests **précisément** (`parsing_blocks_if_else_ok`, `vm_stack_grows_safely`).  
- **Coverage** : cibler les chemins critiques (compilo, vm, stdlib).  
- **CI** (`.github/workflows/ci.yml`) :
  - build + lints + tests + docs (sur PR/push).  
  - jobs Linux + macOS (Windows si pertinent).  
- **Rapports** perf comparatifs sur `perf.yml` (micro/macro benchmarks).

Scripts utiles :
```bash
./scripts/test.sh          # paquet courant
./scripts/test_all.sh      # test suite monorepo
./scripts/ci_check.sh      # lint + fmt + build
```

---

## Benchmarks & perfs

- **Micro** (`benchmarks/micro`) : latences/opérations unitaires (hashes, alloc, parse).  
- **Macro** (`benchmarks/macro`) : programmes Vitte “réels”.  
- **Règles** : machine “calme”, répéter, épingler CPU, pas de turbo variable, exporter CSV.  
- **Budget** perf : toute régression > 3 % sur hot path = **bloquante** (besoin d’argument + bench A/B).  
- Profiler : `tools/vitte-profile` + outils de la plateforme.

---

## Changement de langage (RFCs)

- **Obligatoire** pour : syntaxe, sémantique, modèle d’emprunts, ABI/FFI, bytecode VM.  
- Process :
  1. Fork `rfcs/0000-template.md` → `rfcs/NNNN-titre.md`.  
  2. Discussion (issue ou Discussion).  
  3. Prototype derrière feature-flag (`--feature rfc-NNNN`) + tests.  
  4. **Stabilisation** : gel 2–4 semaines + retours terrain.  
  5. Promotion `experimental → preview → stable` (avec docs).

- **Refus poli** si : ajout casse la lisibilité, coûts cachés, pas de use-case convaincant, surface d’attaque FFI accrue.

---

## Stabilité, SemVer & Éditions

- **SemVer** : MAJOR.MINOR.PATCH sur outils/crates publics.  
- **Niveaux** :  
  - `stable` : gelée, rétro-compat (patch/minor).  
  - `preview` : plausible, peut bouger.  
  - `experimental` : aucune garantie.  
- **Éditions** (ex. 2025, 2026) : regroupent les ruptures, guide de migration fourni.

---

## Process de dépréciation

1. Marquer `@deprecated("raison, remplacement, date")` (preview attrs) ou doc “Deprecated”.  
2. Warnings en `preview`, hard-error **à l’édition suivante**.  
3. Mentionner dans **CHANGELOG** + **stdlib.md** + exemples.  
4. Fournir un **fix-it** (docs ou script migrateur si possible).

---

## Sécurité & signalements

- **Ne publie pas** les vulnérabilités en issue publique.  
- Contact **privé** : `security/` (procédure détaillée) ou email mainteneurs (si listé).  
- Délai de réponse : 72 h ouvrées.  
- Corrections backportées sur branches `release/*` si besoin.

---

## Plugins, FFI & ABI

- **FFI C** : `extern(c)`, échanges en **ptr+len**, ownership **clair**.  
- Enums riches **non** exportés directement. Préfère **codes + payloads**.  
- Plugins dynamiques : vtable versionnée (voir `ffi.md`).  
- Toute modif d’ABI = **RFC + bump MINOR/MAJOR**.

---

## Docs & site

- Édits dans `docs/` : `index.md`, `getting-started.md`, `language-spec.md`, `stdlib.md`, `ffi.md`, `contributing.md`.  
- Génération : `./scripts/gen_docs.sh` → build local + GH Pages via `pages-docs.yml`.  
- **Exemples** : courts, compilables, avec commentaires.  
- Orthographe FR cohérente, ton clair, pas de jargon gratuit.

---

## Checklists par domaine

### A) **Stdlib** (nouveau module)
- [ ] Nom court (`stringx`, `mathx`, …) + but clair.  
- [ ] API publique sans `panic!` pour erreurs attendues → `Result`.  
- [ ] **Stabilité** tag : `experimental`/`preview`/`stable`.  
- [ ] Tests unitaires + intégration (happy + erreurs).  
- [ ] Bench (si hot path).  
- [ ] Doc bloc + entrée dans `docs/stdlib.md` (exemples).  
- [ ] Exemple minimal dans `examples/`.

### B) **Compiler**
- [ ] Grammaire (MAJ `language-spec.md` si langage).  
- [ ] Tests parsing & typing (positifs/négatifs).  
- [ ] Performance (pas de régression sur `benchmarks/micro`).  
- [ ] Pas d’`unwrap` en prod code ; erreurs contextualisées.  
- [ ] Review **2 pairs**.

### C) **VM / Runtime**
- [ ] Tests stack/heap, limites, OOM.  
- [ ] Vérifs d’alignement & UB documentées.  
- [ ] Bench “macro” (programmes réels).  
- [ ] Compat bytecode (bump version si besoin + outil de migration).  
- [ ] Review **2 pairs**.

### D) **Outils (vitc, vitpm, etc.)**
- [ ] UX CLI cohérente (`--help` exhaustif).  
- [ ] Logs corrects (`--verbose`), codes retours clairs.  
- [ ] Tests e2e si possible (snapshots ok).  
- [ ] Docs dans `docs/` + entrée `index.md`.

### E) **Docs**
- [ ] TOC, titres hiérarchisés, snippets compilables.  
- [ ] Pas d’images raster sans source.  
- [ ] Liens internes relatifs (`../`).

---

## Code de conduite

- **Respect** et **bienveillance**. Pas d’attaques perso, pas de harcèlement, zéro tolérance.  
- Assume la bonne foi, mais demande des repros **concrets**.  
- Les mainteneurs peuvent fermer les discussions hors sujet ou irrespectueuses.  
- Conflits : on migre en RFC/issue technique, on bench, on tranche.

---

## Licence & Droit

- **Double licence** : MIT **ou** Apache-2.0 (au choix du downstream). Voir `LICENSE`.  
- **CLA** : aucun. **DCO** recommandé : `git commit -s`.  
- En contribuant, tu assures pouvoir licencier ton code sous ces termes.

---

## FAQ contributeur

**Q. Je peux envoyer une énorme PR “fourre-tout” ?**  
A. Non. Découpe. Une idée = une PR → review plus rapide.

**Q. Ma PR rouge en CI pour un warning Clippy ?**  
A. Corrige ou `#[allow]` **justifié** au scope minimal.

**Q. Comment “générer l’arborescence markdown” depuis VS Code ?**  
A. Quick & dirty :
```bash
git ls-tree -r --name-only HEAD | sed 's|^|- |' > docs/tree.md
```
Version plus fancy dans `docs/arborescence.md`.

**Q. J’ajoute une feature de langage ?**  
A. Passe par **RFC** (voir section dédiée) + garde derrière un **feature-flag**.

**Q. Comment lancer seulement les tests VM ?**  
A.
```bash
cargo test -p vitte-vm
```
ou
```bash
./scripts/test.sh vm
```

---

## Annexe : hooks Git utiles

`.git/hooks/pre-push` (exemple minimal)
```bash
#!/usr/bin/env bash
set -euo pipefail
./scripts/ci_check.sh
./scripts/test.sh
```
Rends-le exécutable : `chmod +x .git/hooks/pre-push`.

---

> _“Fais simple, mesure tout, documente bien. Le reste suivra.”_  
Merci d’élever Vitte avec nous — PR par PR, benchmark par benchmark.
