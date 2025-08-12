# Backends — REAL builds (2025-08-12T07:21:47.145644Z)

## Cranelift
```bash
# Linux/macOS
cargo run -p vitte-backend-demo --features real -- --backend cranelift --imm 2025
```
Dépendances tirées automatiquement: `cranelift-{codegen,frontend,module,object}`, `target-lexicon`.

## LLVM (Inkwell)
```bash
# Prérequis: LLVM installé et visible (llvm-config), ex: brew install llvm
cargo run -p vitte-backend-demo --features real -- --backend llvm --imm 2025
```
Dépendance: `inkwell` (liée à ta version LLVM locale).

> Dans les deux cas, on **linke via `cc`** pour produire un **.vitx** et on `strip` derrière.
