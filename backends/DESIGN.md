# Vitte Backends — Design Notes (2025-08-12T07:14:17.932250Z)
- **API v2**: MIR SSA avec blocs, SSA regs, types, callconv, target.
- **Cranelift/LLVM**: même contrat `Backend` (compile_fn/link), wiring interchangeable.
- **Bootstrap**: C shim + `cc` pour produire un `.vitx` dès maintenant.
- **À brancher**: cranelift-codegen / inkwell pour codegen réel; select linker (lld/link/wasm-ld).

## Pipeline
MIR (builder) → lower (pseudo IR) → **(futur)** CLIF/LLVM → objet `.o` → link `.vitx` (+ .vmeta).

## Tests
Ajoutez des tests unitaires dans chaque backend (lower, abi, isa) et une intégration
dans `backends/examples/mini` qui vérifie que l'artifact `.vitx` est produit.
