# Changelog
Toutes les modifications notables de l’extension **Vitte Language Support** seront documentées ici.

Le format est basé sur [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/)
et ce projet adhère au [SemVer](https://semver.org/lang/fr/).

> Compatibilité VS Code : **≥ 1.85.0**  
> Plateformes : **Linux / macOS / Windows**  
> Langage supporté : **.vitte**, **.vit**

---

## [Unreleased]
### Ajouté
- Détection auto du `vitc` dans les environnements *monorepo* (multi-workspace).
- Support des *code actions* de correction rapide (ex : import manquant, formatage d’un bloc).
- Surbrillance des *todo-notes* configurable (`vitte.highlightTodo`).

### Modifié
- Amélioration des performances de diagnostic sur de gros projets (> 2k fichiers).
- Lancement parallèle du *formatter* pour documents non actifs (expérimental).

### Corrigé
- Nettoyage des diagnostics fantômes lors de renommages de fichiers.
- Cas bord pour chemins Windows `file://C:/...` dans la sortie CLI.

### Sécurité
- Durcissement de l’appel des outils externes (évasion d’arguments).

---

## [0.2.0] - 2025-08-15
### Ajouté
- **Gram­maire TM** enrichie : blocs `impl`, generics `T[U]`, doc-comments `//!`, attributs `@attr(...)`.
- **Snippets** étendus (CLI, HTTP, fs atomic, channels, tracing, pagination, uuid v7, cache LRU, kvstore TTL, FFI).
- **Diagnostics robustes** (`src/diag.ts`) :
  - Essai `vitc --format=json`, *fallback* texte type gcc/clang.
  - Groupement par fichier, ranges corrects (1-based → 0-based), `diag.source = "vitc"`.
- **Formatage** (`src/format.ts`) :
  - Stratégie multi-essais : `--stdin` → `-` → pipe simple.
  - Timeout et messages d’erreur *friendly* (PATH, permissions).
- **Commandes** :
  - `Vitte: Run`, `Build`, `Test`, `Check`, `Format Document`.
  - Terminal unique **“Vitte”** avec `cd` automatique vers la racine de projet (détection `vitte.toml`, `.git`, etc.).
- **Status bar** : bouton “Vitte” → `Check` direct.
- **Paramètres** (`package.json > contributes.configuration`) :
  - `vitte.vitcPath`, `vitte.fmtPath`, `vitte.enableDiagnostics`,
    `vitte.checkArgs`, `vitte.buildArgs`, `vitte.runArgs`, `vitte.testArgs`,
    `vitte.formatOnSave` (par défaut **true**).

### Modifié
- Fournisseur de formatage : retourne un *full document edit* unique pour stabilité.
- *Format-on-save fallback* : s’active si `vitte.formatOnSave=true` **et** `editor.formatOnSave=false` (pas de double format).

### Corrigé
- Quoting cross-platform des arguments (PowerShell/cmd/bash/zsh).
- Ranges invalides quand le compilateur ne renvoie qu’une position (colonne par défaut → 1).
- Nettoyage des diagnostics lors d’un *check* “tout vert”.

### Déprécié
- Rien.

### Supprimé
- Rien.

### Sécurité
- Kill forcé du formateur sur dépassement de délai (évite les pendings).

---

## [0.1.2] - 2025-07-30
### Ajouté
- Surbrillance des *doc comments* `///` et `/** ... */`.
- *Keybindings* par défaut :
  - **Ctrl+Alt+C** `Vitte: Check`
  - **Ctrl+Alt+F** `Vitte: Format Document`
  - **Ctrl+Alt+B/R/T** Build/Run/Test

### Modifié
- Messages d’erreur plus clairs quand `vitte-fmt` n’est pas trouvé.

### Corrigé
- Problème de *folding* sur régions `// region` / `// endregion`.

---

## [0.1.1] - 2025-07-12
### Ajouté
- Icône d’extension (`media/icon.png`).
- Snippets de base (`do`, `struct`, `enum`, `match`, `trait`, `impl`, `extern(c)`).

### Corrigé
- Détection de nombres hex/bin/oct avec underscores.

---

## [0.1.0] - 2025-06-28
### Ajouté
- Première publication.
- Coloration syntaxique minimale (`vitte.tmLanguage.json`).
- Fournisseur de formatage initial (`vitte-fmt --stdin`).
- Diagnostics basiques via `vitc check` (regex texte).
- Commandes de base et configuration initiale.

---

## Guide de migration
- **0.2.0** : aucun changement de paramètre cassant.  
  Si tu utilisais un formateur custom, vérifie que `--stdin` est supporté ; sinon, l’extension basculera sur `-` puis pipe.

---

## Notes de version / Publication
- Packager en local :
  ```bash
  npm run build
  npm run package   # produit un .vsix
  code --install-extension vitte-*.vsix
