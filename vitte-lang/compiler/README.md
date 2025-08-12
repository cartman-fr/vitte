# vitte-compiler

Un **orchestrateur de compilation** pour vitte-lang.

## Idée
- Pipeline modulaire (parse → résolve → typer → MIR → backend).
- Backend **prêt à l’emploi**: `bytecode-cli` qui délègue au binaire `vitte` existant pour produire `.vbc`.
- API stable que les outils (CLI, LSP, builder) peuvent réutiliser.

## Utilisation
```rust
use vitte_compiler::{Compiler, CompilerConfig, OutputKind};

let cfg = CompilerConfig::default();
let c = Compiler::new(cfg);
let out_dir = std::env::temp_dir();
let prod = c.compile_str(r#"print("hey")"#, &out_dir, OutputKind::BytecodeVbc)?;
std::fs::write(out_dir.join("a.vbc"), prod.output.unwrap())?;
# Ok::<(), color_eyre::Report>(())
```
> Pour les tests, définis `VITTE_BIN=/chemin/vers/vitte` (ou installe `vitte` dans le PATH).

## Roadmap
- Ajout d’un backend **in-process** (réutilisation directe des modules parse/bytecode v8).
- Passes **Hir/Mir/Infer** optionnelles (features) et hooks `lints`/`incremental`.
- Journal de **diagnostics** riche (fichiers, spans, codes).

## Licence
MIT ou Apache-2.0.