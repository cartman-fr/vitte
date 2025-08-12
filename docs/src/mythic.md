# MYTHIC

- **LLVM runtime complet (listes)** : `len`, `get`, `set`, `push` via `vitte-rt`. 
- **Codegen LLVM** : reconnaissance de `print()` et `len()` (listes), indexation `lst[i]`, et `lst.set(i, v)`.
- **LSP** : lenses dynamiques par statement qui déclenchent un DOT inline focalisé.
- **`--emit llvm-run`** : essaie `lli`, sinon fallback `clang` si `VITTE_RT_PATH` est fourni.

## Exécution avec runtime
```
# Construire le runtime (générer .o/.a en dehors de ce dépôt selon ton toolchain)
export VITTE_RT_PATH=/chemin/vers/libvitte-rt.o   # ou .a
cargo run -p vittc -- --emit llvm examples/llvm_lists.vitte > /tmp/lists.ll
clang /tmp/lists.ll $VITTE_RT_PATH -O2 -o /tmp/lists && /tmp/lists
```
