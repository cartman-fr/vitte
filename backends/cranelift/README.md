# Vitte Backend — Cranelift (Design)

## Objectif
- Compilation rapide en dev, débogage agréable, codegen stable.
- Pipeline: AST -> HIR -> MIR(SSA) -> CLIF -> ISA
- Modes: `debug-fast` (Cranelift), `release-max` (LLVM)

## Plan d'implémentation
1. MIR SSA (phi, br, call, ret, load/store)
2. Lower MIR -> CLIF (instructions: iconst, fconst, iadd, imul, call, ret)
3. Regalloc: laisser Cranelift gérer
4. Link: lld/link.exe, ajout section `.vmeta`

## Prio
- `fn main(){ print("hello") }` -> code natif
- `fn add(i32,i32)->i32` -> ABI C
- `io::print` -> FFI minimal C (puts)
