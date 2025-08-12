# vitte / compiler

Pipeline: **AST -> HIR -> MIR -> (Bytecode|LLVM)**

- AST: lexer+Pratt, `if/then/else`, `fn`, `import`, listes.
- HIR: normalisation simple.
- Resolver: table de symboles (globaux).
- Infer: HM light (stub extensible).
- MIR: instructions simples (Const, Bin, Call, If, Load/Store).
- Backends:
  - Bytecode + VM (stack machine) — exécute.
  - LLVM IR (printf) — imprime ints/strings, arith, if.

Usage rapide:
```bash
cd compiler
cargo build
cargo run -p vittec -- run examples/hello.vitte
cargo run -p vittec -- llc examples/math.vitte | sed -n '1,120p'
```
