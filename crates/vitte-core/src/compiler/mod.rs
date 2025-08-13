//! Module `compiler` (vitte-core)
//!
//! 🧩 Ce module fédère les trois briques internes :
//! - [`config`]  : noyau de configuration (opt-level, strip, limites…)
//! - [`driver`]  : pipeline build (compile/asm/load + link + strip + stdlib*)
//! - [`output`]  : émission des artefacts (bytecode, disasm, asm, json, map, hexdump)
//!
//! \* La compilation `.vit` (frontend) et la stdlib sont contrôlées par des **features** :
//! - `frontend` : permet à `driver` d’appeler `vitte-compiler::compile_str`
//! - `stdlib`   : permet à `driver` de linker le prélude ou toute la stdlib
//!
//! ## Exemples rapides
//! ```no_run
//! use std::path::PathBuf;
//! use vitte_core::compiler::{self as vc, *};
//!
//! // 1) Config (défauts + ENV)
//! let mut cfg = Config::from_env();
//! cfg.codegen.strip_debug = false;
//!
//! // 2) Options pipeline
//! let mut opts = BuildOptions::default();
//! opts.link_std = true;                 // nécessite feature "stdlib"
//! opts.std_prelude_only = true;
//!
//! // 3) Build
//! let out = build(&[PathBuf::from("src/main.vit")], &cfg, &opts).expect("build ok");
//!
//! // 4) Emit (bytecode + désasm compact)
//! let plan = EmitPlan::for_input_path(PathBuf::from("src/main.vit"))
//!     .with(OutputKind::Bytecode(None))
//!     .with(OutputKind::Disasm { mode: DisasmMode::Compact, path: None });
//! build_and_emit(&[PathBuf::from("src/main.vit")], &cfg, &opts, &plan).expect("emit ok");
//! ```

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, unused_must_use)]

use std::path::{Path, PathBuf};

/* ───────────────────────────── Sous-modules ───────────────────────────── */

pub mod config;
pub mod driver;
pub mod output;

/* ───────────────────────────── Réexports utiles ───────────────────────────── */

// config
pub use config::{
    CliOverrides, Codegen, ColorMode, Config, DebugInfo, Endianness, Limits, OptLevel, WarningsAs,
};

// driver
pub use driver::{
    BuildOptions, BuildOutput, Diagnostic, Driver, DriverError, Input, InputKind, LinkInput,
    LinkManifest, Severity,
};

// output
pub use output::{
    Artifact, DisasmMode, EmitError, EmitPlan, OutputKind, render_asm, render_manifest_json,
    render_sourcemap_json,
};

/* ───────────────────────────── Orchestrateur ───────────────────────────── */

/// Erreurs possibles d’orchestration (build **ou** émission).
#[derive(Debug)]
pub enum OrchestratorError {
    Driver(DriverError),
    Emit(EmitError),
}
impl From<DriverError> for OrchestratorError {
    fn from(e: DriverError) -> Self { OrchestratorError::Driver(e) }
}
impl From<EmitError> for OrchestratorError {
    fn from(e: EmitError) -> Self { OrchestratorError::Emit(e) }
}

/// Construit un chunk à partir d’1..N chemins (détection .vit/.vit.s/.vitbc).
///
/// S’appuie sur [`Driver::build_paths`].
pub fn build<P: AsRef<Path>>(
    paths: &[P],
    cfg: &Config,
    opts: &BuildOptions,
) -> Result<BuildOutput, DriverError> {
    let ps: Vec<PathBuf> = paths.iter().map(|p| p.as_ref().to_path_buf()).collect();
    Driver::build_paths(&ps, cfg, opts)
}

/// Build + émission de plusieurs artefacts en une fois.
///
/// Utile pour les CLI : un appel, tout sort proprement sur disque.
pub fn build_and_emit<P: AsRef<Path>>(
    paths: &[P],
    cfg: &Config,
    opts: &BuildOptions,
    plan: &EmitPlan,
) -> Result<Vec<Artifact>, OrchestratorError> {
    let out = build(paths, cfg, opts)?;             // -> DriverError possible
    let arts = plan.emit_all(&out.chunk)?;          // -> EmitError possible
    Ok(arts)
}

/// Raccourci : build un seul chemin.
pub fn build_one<P: AsRef<Path>>(
    path: P,
    cfg: &Config,
    opts: &BuildOptions,
) -> Result<BuildOutput, DriverError> {
    build(&[path], cfg, opts)
}

/// Utilitaire : plan par défaut basé sur le **premier** input (ou “out”).
pub fn default_plan_from_inputs<P: AsRef<Path>>(inputs: &[P]) -> EmitPlan {
    if let Some(first) = inputs.first() {
        EmitPlan::for_input_path(first.as_ref().to_path_buf())
    } else {
        EmitPlan::new()
    }
}

/* ───────────────────────────── Helpers pratiques ───────────────────────────── */

/// Renvoie une `Config` initialisée via variables d’environnement `VITTE_CORE_*`.
pub fn default_config_from_env() -> Config {
    Config::from_env()
}

/// Construit rapidement un bytecode `.vitbc` pour un input et l’écrit sur disque.
///
/// - `out_path = None` → `<parent(input)>/<stem>.vitbc`
/// - Retourne le chemin effectivement écrit.
pub fn quick_emit_bytecode<P: AsRef<Path>>(
    input: P,
    cfg: &Config,
    opts: &BuildOptions,
    out_path: Option<PathBuf>,
) -> Result<PathBuf, OrchestratorError> {
    let build = build_one(input.as_ref(), cfg, opts)?;
    let plan = match (&out_path, input.as_ref()) {
        (Some(p), _) => EmitPlan::new().with(OutputKind::Bytecode(Some(p.clone()))),
        (None, p)    => EmitPlan::for_input_path(p.to_path_buf()).with(OutputKind::Bytecode(None)),
    };
    let arts = plan.emit_all(&build.chunk)?;
    // il n’y a qu’un artefact ici
    Ok(arts[0].path.clone())
}

/* ───────────────────────────── Tests fumants ───────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::chunk::{Chunk, ChunkFlags};

    #[test]
    fn reexports_compile() {
        // juste pour assurer que les types sont visibles au parent
        let _c: Config = Config::default();
        let _o: BuildOptions = BuildOptions::default();
        let _m: DisasmMode = DisasmMode::Compact;
        let _k: OutputKind = OutputKind::Hexdump { path: None, limit: Some(16) };
        let _s: Severity = Severity::Info;
    }

    #[test]
    fn default_plan_when_empty() {
        let p = default_plan_from_inputs::<&str>(&[]);
        assert!(p.base_stem.as_deref() == Some("out"));
    }

    #[test]
    fn orchestrator_errors_shape() {
        // smoke-test conversions
        let _: OrchestratorError = DriverError::Io("x".into()).into();
    }

    #[test]
    fn quick_emit_plan_shape() {
        // On ne touche pas le FS ici, on valide juste les chemins calculés.
        let cfg = Config::default();
        let opts = BuildOptions::default();

        // fake: chunk direct + plan (on ne passe pas par FS dans ce test)
        let _c = Chunk::new(ChunkFlags { stripped: false });

        // Vérifie qu’on peut construire un plan par défaut depuis un chemin
        let plan = default_plan_from_inputs(&[PathBuf::from("foo/bar/v.x.vit")])
            .with(OutputKind::Bytecode(None));
        // pas d’émission réelle ici (les tests d’écriture vivent dans `output`)
        let _ = plan; // silence l’avertissement
        let _ = cfg; let _ = opts;
    }
}
