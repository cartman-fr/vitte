use std::path::Path;
use color_eyre::eyre::Result;
use crate::config::CompilerConfig;
use crate::diagnostics::Diagnostic;
use crate::backends::{self, BackendKind};
use crate::backends::bytecode_cli::BytecodeCli;

#[derive(Debug, Clone, Copy)]
pub enum OutputKind {
    BytecodeVbc,
}

/// Produit compilé (+ diagnostics)
#[derive(Debug, Clone)]
pub struct CompileProduct {
    pub output: Option<Vec<u8>>,
    pub diags: Vec<Diagnostic>,
}

/// Orchestrateur principal
pub struct Compiler {
    cfg: CompilerConfig,
    backend: BackendKind,
}

impl Compiler {
    pub fn new(cfg: CompilerConfig) -> Self {
        Self { cfg, backend: BackendKind::BytecodeCli }
    }

    pub fn with_backend(mut self, bk: BackendKind) -> Self { self.backend = bk; self }

    /// Compile un fichier jusqu'au format demandé (actuellement: bytecode v8 via CLI).
    pub fn compile_file(&self, input: &Path, out_dir: &Path, kind: OutputKind) -> Result<CompileProduct> {
        match (self.backend, kind) {
            (BackendKind::BytecodeCli, OutputKind::BytecodeVbc) => {
                let bc = BytecodeCli::new(&self.cfg);
                let bytes = bc.compile_file(input, out_dir)?;
                Ok(CompileProduct{ output: Some(bytes), diags: vec![] })
            }
        }
    }

    /// Compile une chaîne en mémoire.
    pub fn compile_str(&self, source: &str, out_dir: &Path, kind: OutputKind) -> Result<CompileProduct> {
        match (self.backend, kind) {
            (BackendKind::BytecodeCli, OutputKind::BytecodeVbc) => {
                let bc = BytecodeCli::new(&self.cfg);
                let bytes = bc.compile_str(source, out_dir)?;
                Ok(CompileProduct{ output: Some(bytes), diags: vec![] })
            }
        }
    }
}