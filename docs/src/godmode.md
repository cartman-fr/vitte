# GODMODE

- **Upvalues mutables**: captures boxées (`Cell`) + opcode `SetCapture` pour assigner dans les lambdas.
- **Runtime LLVM**: crate `vitte-rt` avec listes concrètes (`Vec<i64>`), `vitte_list_new`, `vitte_list_push`, `vitte_print_i64`.
- **`--emit llvm-run`**: tente `lli`, sinon tu peux lier via `clang` avec `VITTE_RT_PATH` pointant vers `libvitte-rt`.
- **LSP**: preview DOT (MD helper), focus lenses, légende.

> Note: le parser des lambdas reste expression-centré. Pour muter un upvalue dans une lambda, utilise un builtin helper (prochaines étapes) ou une `Def`.
