# MYTHIC+

- **Strings LLVM** : struct `%vitte.str`, `str_new/len/concat`, `print(str)` ; concat branchée côté IR (note : dispatch typé en cours de finalisation).
- **GC scheduler** : déclenchement périodique plus nerveux + builtin `gc()`.
- **Imports** : cache simple des modules résolus (par chemin canonique) pendant l’exécution.
- **LSP** : lenses par `def` (focus sur le premier stmt du corps), en plus des lenses par statement.

Voir aussi `examples/llvm_strings.vitte` pour un tour rapide.
