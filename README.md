# vitte-lang • titan
- **Fonctions** appelables (bytecode: `Call/Ret`), table de fonctions dans le Chunk.
- **Heap** + **GC mark/sweep léger** (racines: stack + globals).
- **Chaînes** et **listes** allouées sur le heap (handles).
- **VM** avec frames d'appel.
- Imports récursifs (inline), fmt/doc/LSP conservés.

Build:
```
cargo build
cargo run -p vitte-cli -- run examples/hello.vitte
```
