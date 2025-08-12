# Testing backends — 2025-08-12T07:16:47.737843Z
- `cargo test -p vitte-backend-cranelift`
- `cargo test -p vitte-backend-llvm`
- `cargo run -p vitte-backend-mini-demo`
- `cargo run -p vitte-backend-demo -- --backend cranelift --imm 777`
Artifacts: `.vitx` si `cc` est dispo, sinon `.c` shim généré.
