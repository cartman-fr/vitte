# Vitte Language Support — Extension VS Code

> _“Écrire du Vitte sans se battre avec l’éditeur.”_  
Coloration, formatage, diagnostics, snippets & commandes pour **.vitte** / **.vit**.

[![VS Code](https://img.shields.io/badge/VS%20Code-%E2%89%A5%201.85-blue)](#prérequis)
[![License: MIT](https://img.shields.io/badge/License-MIT-lightgrey.svg)](#licence)

---

## Sommaire
- [Pourquoi](#pourquoi)
- [Fonctionnalités](#fonctionnalités)
- [Démo](#démo)
- [Prérequis](#prérequis)
- [Installation](#installation)
  - [Marketplace](#marketplace-recommandé)
  - [.vsix](#fichier-vsix)
  - [Depuis la source](#depuis-la-source)
- [Démarrage rapide](#démarrage-rapide)
- [Compatibilité](#compatibilité)
- [Configuration](#configuration)
  - [Paramètres (table)](#paramètres-table)
  - [Exemples de settings](#exemples-de-settings)
- [Commandes](#commandes)
- [Raccourcis clavier](#raccourcis-clavier)
- [Diagnostics (Problems)](#diagnostics-problems)
- [Formatage](#formatage)
- [Snippets inclus](#snippets-inclus)
- [Détection de projet](#détection-de-projet)
- [FAQ](#faq)
- [Dépannage](#dépannage)
- [Limitations connues](#limitations-connues)
- [Développement de l’extension](#développement-de-lextension)
- [Publication / Release](#publication--release)
- [Roadmap](#roadmap)
- [Confidentialité & télémétrie](#confidentialité--télémétrie)
- [Crédits](#crédits)
- [Licence](#licence)

---

## Pourquoi
Parce qu’un bon langage mérite un bon outillage. Cette extension offre des bases **solides et rapides** : coloration, **formateur**, **diagnostics**, **snippets**, commandes **build/run/test** et un **status bar** pour aller vite.

---

## Fonctionnalités
- **Coloration syntaxique (TextMate)** : `do`, `impl`, `trait`, `enum`, `struct`, annotations `@attr(...)`, generics `Type[T]`, doc-comments `///` & `//!`, folding `// region` / `// endregion`.
- **Formatage** via `vitte-fmt` (STDIN), avec *fallbacks* (`-`, pipe) et **timeout**.
- **Diagnostics** via `vitc check` :
  - Essai `--format=json`, fallback texte (*gcc/clang-like*).
  - Ranges précis (1-based → 0-based), regroupement par fichier, `diag.source = "vitc"`.
- **Snippets** : `main`, `do/dor`, `struct`, `enum`, `match`, `trait`, `impl`, `extern(c)`, HTTP+retry, fs atomic, channels, tracing, uuid v7, etc.
- **Commandes** : `Vitte: Run • Build • Test • Check • Format Document`.
- **Status bar** : bouton **Vitte** → `Check` direct.
- **Format-on-save** fallback même si `editor.formatOnSave` est **OFF**.

---

## Démo
> (Place ici un GIF/PNG : `media/demo.gif` montrant formatage + diagnostics.)

```text
src/app.vitte:12:8: error: undefined symbol 'foo'
src/lib.vitte:44: warning: unused variable 'tmp'
```

---

## Prérequis
- **VS Code ≥ 1.85.0**
- **Outils Vitte** dans le `PATH` :
  - `vitc` (CLI) — check/build/run/test
  - `vitte-fmt` — formatage

---

## Installation

### Marketplace (recommandé)
1. VS Code → **Extensions** (`Ctrl+Shift+X`)
2. Cherche **Vitte Language Support** → **Install**

### Fichier `.vsix`
```bash
code --install-extension vitte-*.vsix
```

### Depuis la source
```bash
cd editor-plugins/vscode
npm install
npm run build
# F5 dans VS Code → "Extension Development Host"
```

---

## Démarrage rapide
Crée `hello.vitte` :
```vitte
do main() {
  print("Hello, Vitte!")
}
```
- Formater : `Ctrl+Alt+F` ou `Vitte: Format Document`  
- Vérifier : `Ctrl+Alt+C` ou `Vitte: Check`

---

## Compatibilité
| OS | Support | Remarques |
|---|---|---|
| Linux (x64/arm64) | ✅ | Recommandé |
| macOS (Intel / Apple Silicon) | ✅ | Vérifier le PATH des binaires |
| Windows 10/11 (x64) | ✅ | Quoting géré, privilégier chemins sans espaces |

---

## Configuration

### Paramètres (table)
| Clé | Type | Défaut | Description |
|---|---|---:|---|
| `vitte.vitcPath` | string | `"vitc"` | Chemin du compilateur/CLI. |
| `vitte.fmtPath` | string | `"vitte-fmt"` | Chemin du formateur. |
| `vitte.enableDiagnostics` | boolean | `true` | Active `vitc check` au save/ouverture. |
| `vitte.checkArgs` | string[] | `["check"]` | Arguments par défaut pour `Check`. |
| `vitte.buildArgs` | string[] | `["build"]` | Arguments par défaut pour `Build`. |
| `vitte.runArgs` | string[] | `["run"]` | Arguments par défaut pour `Run`. |
| `vitte.testArgs` | string[] | `["test"]` | Arguments par défaut pour `Test`. |
| `vitte.formatOnSave` | boolean | `true` | Fallback si `editor.formatOnSave` est OFF. |

### Exemples de settings
```jsonc
// .vscode/settings.json (workspace)
{
  "vitte.vitcPath": "vitc",
  "vitte.fmtPath": "vitte-fmt",
  "vitte.enableDiagnostics": true,
  "vitte.checkArgs": ["check", "--format=json"],
  "vitte.formatOnSave": true
}
```

---

## Commandes
- **Vitte: Check** — lance `vitc check` et alimente **Problems**.  
- **Vitte: Build** — `vitc build` dans la racine du projet.  
- **Vitte: Run** — `vitc run`.  
- **Vitte: Test** — `vitc test`.  
- **Vitte: Format Document** — formate le buffer courant avec `vitte-fmt`.

---

## Raccourcis clavier
- `Ctrl+Alt+C` → **Vitte: Check**  
- `Ctrl+Alt+F` → **Vitte: Format Document**  
- `Ctrl+Alt+B` → **Vitte: Build**  
- `Ctrl+Alt+R` → **Vitte: Run**  
- `Ctrl+Alt+T` → **Vitte: Test**  
> Reconfigurable via **Keyboard Shortcuts**.

---

## Diagnostics (Problems)
- Essai `--format=json` (plus précis), sinon parsing texte robuste.
- Ranges : conversion **1-based → 0-based** ; `diag.source = "vitc"`.
- Nettoyage des diagnostics fantômes lors de renommages/fermetures.

---

## Formatage
- `vitte-fmt` via **STDIN** (préféré). Fallbacks : `-`, puis pipe simple.
- **Timeout** pour éviter les blocages.
- Si `stdout` vide et code=0 → pas de changement → on garde le buffer.

Astuce (format-on-save même si l’éditeur ne formate pas) :
```json
{
  "vitte.formatOnSave": true,
  "editor.formatOnSave": false
}
```

---

## Snippets inclus
- **Base** : `main`, `do`, `dor` (Result), `struct`, `enum`, `trait`, `impl`, `match`.
- **IO/Sys** : fs read/write, fs atomic, process spawn, sleep.
- **Réseau** : http get/post, retry policy, rate limiter.
- **Concurrence** : channels, try_recv, taskpool, scheduler.
- **Observabilité** : log, metrics, tracing spans, pagination.
- **Utilitaires** : uuid v4/v7, idgen, random, stringx, mathx, csv/ini/yaml_lite, checksum, rle.
- **FFI** : `extern(c)`, pointeur+longueur, gestion codes d’erreur.

> Snippets dans `snippets/vitte.code-snippets`.

---

## FAQ
**Q : Pourquoi rien n’apparaît dans Problems ?**  
R : Vérifie `vitte.enableDiagnostics`, puis lance `Vitte: Check`. Regarde `Output → Vitte` si disponible, ou la Console DevTools.

**Q : Le formatage ne marche pas.**  
R : Vérifie le `PATH` et `vitte.fmtPath`. Teste en terminal : `echo 'do main(){}' | vitte-fmt --stdin`.

**Q : Sous Windows, les chemins avec espaces échouent.**  
R : Utilise des chemins sans espaces ou configure des chemins absolus dans les settings. L’extension fait du quoting mais certains shells sont tatillons.

---

## Dépannage
- “**vitte-fmt introuvable**” → configure `vitte.fmtPath` ou ajoute-le au `PATH`.
- “**vitc introuvable**” → configure `vitte.vitcPath` ou installe `vitc` globalement.
- **Permissions** (Linux/macOS) → `chmod +x vitte-fmt` / `vitc`.
- **Timeout** → augmente le délai côté outils si possible ; sinon investiguer les entrées qui bloquent.
- **Monorepo** → vérifie la racine détectée (fichier `vitte.toml`, `.git`, etc.).

---

## Limitations connues
- Sans JSON fiable, le parsing texte est moins précis.
- Le formateur doit accepter **STDIN** ou `-`.
- Pas d’LSP complet (encore).

---

## Développement de l’extension
```bash
cd editor-plugins/vscode
npm install
npm run build           # compile TypeScript → out/
# F5 dans VS Code (Extension Development Host)
```
Scripts : `npm run build`, `npm run watch`, `npm run package`

Fichiers clés : `src/extension.ts`, `src/format.ts`, `src/diag.ts`, `src/utils.ts`, `syntaxes/vitte.tmLanguage.json`, `language-configuration.json`, `snippets/vitte.code-snippets`

---

## Publication / Release
- Suivre **SemVer** + `CHANGELOG.md` (Keep a Changelog).
- Packager :
  ```bash
  npm run package
  code --install-extension vitte-*.vsix
  ```
- CI : publier au tag `v*.*.*`.

---

## Roadmap
- LSP dédié (autocomplétion, go-to-definition, hover).
- Code actions (imports auto, quick-fixes).
- Inlay hints / symbol outline amélioré.

---

## Confidentialité & télémétrie
- L’extension **n’envoie aucune donnée**.
- Exécutions de `vitc`/`vitte-fmt` **locales**.
- Respect des paramètres de télémétrie VS Code.

---

## Crédits
- Icône & branding : équipe Vitte.  
- Merci aux contributrices et contributeurs pour les retours & PRs ⚡

---

## Licence
MIT © Vitte contributors
