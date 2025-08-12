//! vitte-compiler — orchestrateur de compilation pour vitte-lang.
//! - Pilote la pipeline (parse → résolve → typer → MIR → backend)
//! - Fournit un backend **pratique**: `bytecode-cli` qui appelle le binaire `vitte` existant.
//! - API stable orientée outil (CLI, LSP, build system).

pub mod config;
pub mod diagnostics;
pub mod pipeline;
pub mod backends;
mod util;

pub use config::CompilerConfig;
pub use diagnostics::{Diagnostic, Severity};
pub use pipeline::{Compiler, OutputKind, CompileProduct};