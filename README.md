# vitte-lang • omega
- VM avec variables globales (LoadG/StoreG), listes, len, if (Jz/Jmp).
- Bytecode compile Var/Assign/If/Lists/Arith/Print/Len.
- CLI résout `import "path.vitte"` récursivement (inlining).
- LLVM IR: arith, print int & string, If en branches.
- LSP squelette, fmt, doc, infer simple.

Build:
```
cargo build
cargo run -p vitte-cli -- run examples/hello.vitte
```
