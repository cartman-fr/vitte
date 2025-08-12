# LLVM (IR + exécution)

- IR: `vittc --emit llvm file.vitte > out.ll`
- Exécution (si `lli` est disponible) : `vittc --emit llvm-run file.vitte`

> Pour les listes, le code appelle des stubs runtime (`vitte-rt`). Pour une exécution complète, lie le binaire avec `vitte-rt` ou exécute via `lli` si ton programme ne touche pas aux listes.
