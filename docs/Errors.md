# Errors v1.3 — Zero-cost & Practical

- `Result<T,E>` partout (I/O, FFI, parsing).
- `SmallErr` (<= 32 B) : code, tag, payload court. Aligne 2 words → cheap.
- `RichErr` dev-only : message heap + backtrace (strip en release).
- `?` se compile en test + jump (SSA), pas d’unwind.
- `ensure!(cond, E)`, `bail!(E)` = sucre → branches déterministes.
- `panic!` : abort en release ; debug `-Zeh_unwind` autorise backtrace.
- Tables d’erreurs (codes stables) pour ABI & logs parsables.

## Codes d'erreurs stables
- IO: EIO, ENOENT, EACCES, ETIMEOUT
- NET: ECONN, EREFUSED, ETLS
