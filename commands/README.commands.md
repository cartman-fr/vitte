
# vitte-lang • CLI /commands

Ce dossier regroupe les sous-commandes du binaire `vitte`.

## Sous-commandes incluses
- `run` : exécuter un fichier .vitte via l’interpréteur.
- `bc`  : compiler en bytecode (.vbc).
- `vm`  : exécuter un .vbc sur la VM.
- `fmt` : formatter le code source.
- `repl`: REPL interactive.
- `tests`: exécution de fichiers avec attentes `# EXPECT:`.
- `bench`: micro-benchs simples.
- `doc` : générer la documentation statique (wiki/offline).
- `llc` : passerelle codegen (LLVM/skeleton).
- `pm`  : gestionnaire minimal de paquets/modules (squelette).
- `completions`: générer l’autocomplétion shell.
- `man` : générer la page man.

Intégration : le module `mod.rs` exporte tous les sous-modules.
