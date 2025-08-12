
# ZENITH

- **Loader AST-first** avec cache persistant (JSON) des exports d'un module, contrôlé via `VITTE_CACHE_DIR`.
- `vittc` et `vitte-run` s'appuient désormais sur le **loader**, pas sur un préprocesseur.
- **CLI cache** : `vitte-cache --clear`.
- **CI GitHub** : build & tests pour le workspace.
- **Exemples multi-modules** : `examples/mod_a.vitte`, `examples/mod_b.vitte`.
