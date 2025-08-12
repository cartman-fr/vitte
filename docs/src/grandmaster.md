
# GRANDMASTER

- **vitte-run**: lance un fichier au choix en VM (`--backend vm`), émet l'IR (`--backend llvm`) ou tente l'exécution via `lli` (`--backend llvm-run`).
- **Builtin `str(x)`**: conversion i64 → string côté LLVM (`vitte_str_from_i64`).
- **Kitchen sink example**: `examples/kitchen_sink.vitte` touche un peu tout (imports, closures mutables, listes, strings, slices).
