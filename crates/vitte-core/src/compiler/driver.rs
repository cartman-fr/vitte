//! driver.rs — Pilote de construction (compile/asm/load/link) pour Vitte.
//!
//! Entrées supportées :
//!  - .vit     → compile (feature "frontend": vitte-compiler), fallback erreur sinon
//!  - .vit.s   → assemble (via vitte_core::asm)
//!  - .vitbc   → charge tel-quel (from_bytes)
//!
//! Multi-fichiers → link : concat des opcodes + **dédup** des constantes,
//! réécriture des `LoadConst`, fusion optionnelle des infos debug, strip éventuel.
//!
//! Intègre la **stdlib** si feature "stdlib" activée : prélude seul ou tout.
//!
//! ❗ Sans dépendance externe : erreurs/diagnostics minimalistes (std).

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, unused_must_use)]

use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::bytecode::{
    chunk::{Chunk, ChunkFlags, DebugInfo},
    ConstValue,
    op::Op,
};
use super::config::{Config, CliOverrides};

#[cfg(feature = "frontend")]
use crate as vitte_core; // alias local, et on appellera vitte_compiler
#[cfg(feature = "frontend")]
use vitte_compiler as compiler;

#[cfg(feature = "stdlib")]
use vitte_stdlib as stdlib;

/* ─────────────────────────── Types publics ─────────────────────────── */

/// Type d’entrée détecté ou imposé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind { SourceVit, Asm, Bytecode }

/// Élément d’entrée (chemin + genre).
#[derive(Debug, Clone)]
pub struct Input {
    pub path: PathBuf,
    pub kind: InputKind,
}

/// Gravité des diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity { Error, Warning, Info }

/// Diagnostic minimaliste.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub col: Option<u32>,
}

/// Options de build “driver”.
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Laisse le driver détecter par extension si None.
    pub enforce_kind: Option<InputKind>,
    /// `true` → linke la stdlib.
    pub link_std: bool,
    /// Linke uniquement le **prélude** de la stdlib si `link_std` est vrai.
    pub std_prelude_only: bool,
    /// Fusionner les infos de debug lors du lien (si non strip).
    pub merge_debug: bool,
    /// Symbole d’entrée à valider/annoter après lien.
    pub entry_symbol: Option<String>,
    /// Vérifier un round-trip to_bytes→from_bytes à la fin.
    pub verify_roundtrip: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            enforce_kind: None,
            link_std: false,
            std_prelude_only: true,
            merge_debug: true,
            entry_symbol: None,
            verify_roundtrip: false,
        }
    }
}

/// Résultat de build (driver).
#[derive(Debug)]
pub struct BuildOutput {
    pub chunk: Chunk,
    pub manifest: LinkManifest,
    pub diagnostics: Vec<Diagnostic>,
}

/// Statistiques/mapping du lien (utile pour logs / tests).
#[derive(Debug, Clone)]
pub struct LinkManifest {
    pub inputs: Vec<LinkInput>,
    pub total_ops: usize,
    pub total_consts_before: usize,
    pub total_consts_after: usize,
    pub merged_debug_files: usize,
    pub merged_debug_symbols: usize,
    pub entry: Option<String>,
    pub hash: u64,
}

#[derive(Debug, Clone)]
pub struct LinkInput {
    pub file: String,
    pub ops: usize,
    pub consts: usize,
}

/* ───────────────────────────── Erreurs ───────────────────────────── */

#[derive(Debug)]
pub enum DriverError {
    Io(String),
    InvalidInput(String),
    Unsupported(String),
    Compile(String),
    Assemble(String),
    Load(String),
    Link(String),
    Verify(String),
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DriverError::*;
        match self {
            Io(s)         => write!(f, "io: {s}"),
            InvalidInput(s)=> write!(f, "entrée invalide: {s}"),
            Unsupported(s)=> write!(f, "non supporté: {s}"),
            Compile(s)    => write!(f, "compile: {s}"),
            Assemble(s)   => write!(f, "assemble: {s}"),
            Load(s)       => write!(f, "load: {s}"),
            Link(s)       => write!(f, "link: {s}"),
            Verify(s)     => write!(f, "verify: {s}"),
        }
    }
}

impl From<io::Error> for DriverError {
    fn from(e: io::Error) -> Self { DriverError::Io(e.to_string()) }
}

/* ───────────────────────────── Driver ───────────────────────────── */

/// Pilote : utilitaires statiques.
pub struct Driver;

impl Driver {
    /* ----- Entrées & détection ----- */

    /// Détecte `InputKind` par extension (.vit / .vit.s / .vitbc).
    pub fn detect_kind(path: &Path) -> Option<InputKind> {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
        if ext == "vit" { return Some(InputKind::SourceVit); }
        if ext == "vitbc" { return Some(InputKind::Bytecode); }
        if ext == "s" {
            // Heuristique : foo.vit.s / .vs ? On accepte .s si parent .vit.s
            if path.file_stem()
                .and_then(|s| s.to_str())
                .map(|name| name.ends_with(".vit"))
                .unwrap_or(false) { return Some(InputKind::Asm); }
            // sinon, on essaie le contenu plus tard si enforce_kind=None
            return Some(InputKind::Asm);
        }
        // .vit.s (double extension) : cas fréquent
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if name.ends_with(".vit.s") { return Some(InputKind::Asm); }
        }
        None
    }

    /// Construit une liste d’inputs depuis des chemins. Si `opts.enforce_kind` est Some,
    /// l’utilise pour tous; sinon, détecte au cas par cas.
    pub fn make_inputs(paths: &[PathBuf], opts: &BuildOptions) -> Result<Vec<Input>, DriverError> {
        let mut v = Vec::with_capacity(paths.len());
        for p in paths {
            let k = if let Some(kind) = opts.enforce_kind {
                kind
            } else {
                Self::detect_kind(p).ok_or_else(|| DriverError::InvalidInput(format!(
                    "impossible de déduire le type pour '{}'", p.display()
                )))?
            };
            v.push(Input { path: p.clone(), kind: k });
        }
        Ok(v)
    }

    /* ----- Pipelines haut-niveau ----- */

    /// Pipeline complet pour plusieurs chemins.  
    /// - compile/assemble/load chaque entrée
    /// - (optionnel) link stdlib
    /// - link final + strip/dedup/merge_debug selon `Config`
    pub fn build_paths(paths: &[PathBuf], cfg: &Config, opts: &BuildOptions) -> Result<BuildOutput, DriverError> {
        let inputs = Self::make_inputs(paths, opts)?;
        Self::build_many(&inputs, cfg, opts)
    }

    /// Idem, mais on fournit déjà `Input`.
    pub fn build_many(inputs: &[Input], cfg: &Config, opts: &BuildOptions) -> Result<BuildOutput, DriverError> {
        let mut diags = Vec::<Diagnostic>::new();
        let mut chunks: Vec<(String, Chunk)> = Vec::with_capacity(inputs.len() + 1);

        // 1) Front-ends par entrée
        for it in inputs {
            match it.kind {
                InputKind::SourceVit => {
                    let src = fs::read_to_string(&it.path)
                        .map_err(|e| DriverError::Io(format!("lecture {}: {e}", it.path.display())))?;
                    let ch = Self::compile_source(&src, it.path.file_name().and_then(|s| s.to_str()));
                    match ch {
                        Ok(chunk) => chunks.push((display_of(&it.path), chunk)),
                        Err(e) => return Err(DriverError::Compile(e)),
                    }
                }
                InputKind::Asm => {
                    let src = fs::read_to_string(&it.path)
                        .map_err(|e| DriverError::Io(format!("lecture {}: {e}", it.path.display())))?;
                    let ch = Self::assemble_source(&src)
                        .map_err(|e| DriverError::Assemble(e))?;
                    chunks.push((display_of(&it.path), ch));
                }
                InputKind::Bytecode => {
                    let bytes = fs::read(&it.path)
                        .map_err(|e| DriverError::Io(format!("lecture {}: {e}", it.path.display())))?;
                    let ch = Self::load_chunk(&bytes)
                        .map_err(|e| DriverError::Load(e))?;
                    chunks.push((display_of(&it.path), ch));
                }
            }
        }

        // 2) Stdlib (optionnelle)
        if opts.link_std {
            #[cfg(feature = "stdlib")]
            {
                let std_chunk = if opts.std_prelude_only {
                    stdlib::compile_prelude().map_err(|e| DriverError::Compile(format!("stdlib prelude: {e}")))?
                } else {
                    stdlib::compile_all().map_err(|e| DriverError::Compile(format!("stdlib all: {e}")))?
                };
                chunks.insert(0, (if opts.std_prelude_only { "<stdlib:prelude>" } else { "<stdlib:all>" }.to_string(), std_chunk));
            }
            #[cfg(not(feature = "stdlib"))]
            {
                return Err(DriverError::Unsupported(
                    "link_std demandé, mais la feature `stdlib` n’est pas activée dans vitte-core".into()
                ));
            }
        }

        // 3) Link final
        let (linked, manifest) = Self::link(&chunks, cfg, opts)?;
        let mut chunk = linked;

        // 4) Verify round-trip
        if opts.verify_roundtrip || cfg.codegen.verify_roundtrip {
            let bytes = chunk.to_bytes();
            let chk = Chunk::from_bytes(&bytes).map_err(|e| DriverError::Verify(format!("from_bytes: {e}")))?;
            if chk.compute_hash() != chunk.compute_hash() {
                return Err(DriverError::Verify("hash différent après round-trip".into()));
            }
        }

        // 5) Diagnostics (pour l’instant, rien de sophistiqué ici)
        if cfg.codegen.strip_debug && opts.merge_debug {
            diags.push(Diagnostic {
                severity: Severity::Info,
                message: "debug fusion ignoré car strip activé".into(),
                file: None, line: None, col: None
            });
        }

        Ok(BuildOutput { chunk, manifest, diagnostics: diags })
    }

    /// Pipeline pour une seule entrée.
    pub fn build_one(path: &Path, cfg: &Config, opts: &BuildOptions) -> Result<BuildOutput, DriverError> {
        Self::build_paths(&[path.to_path_buf()], cfg, opts)
    }

    /* ----- Front-ends unitaires ----- */

    /// Compile du **source .vit** (feature `frontend`).
    pub fn compile_source(src: &str, name: Option<&str>) -> Result<Chunk, String> {
        #[cfg(feature = "frontend")]
        {
            let c = compiler::compile_str(src, name.or(Some("<source>")))
                .map_err(|e| format!("{e}"))?;
            Ok(c)
        }
        #[cfg(not(feature = "frontend"))]
        {
            let n = name.unwrap_or("<source>");
            Err(format!("compilation .vit indisponible (feature `frontend` non activée). Fichier: {n}"))
        }
    }

    /// Assemble du **.vit.s** (assembleur core).
    pub fn assemble_source(src: &str) -> Result<Chunk, String> {
        // assembleur exposé par vitte_core::asm
        crate::asm::assemble(src).map_err(|e| format!("{e}"))
    }

    /// Charge un **.vitbc** depuis des bytes.
    pub fn load_chunk(bytes: &[u8]) -> Result<Chunk, String> {
        Chunk::from_bytes(bytes).map_err(|e| format!("{e}"))
    }

    /* ----- Linker (local, sans dépendance externe) ----- */

    /// Linke une liste `(nom, chunk)` en appliquant `Config` + `BuildOptions`.
    pub fn link(inputs: &[(String, Chunk)], cfg: &Config, opts: &BuildOptions)
        -> Result<(Chunk, LinkManifest), DriverError>
    {
        use std::collections::HashMap;

        // Chunk résultat, éventuellement strip plus tard
        let mut out = Chunk::new(ChunkFlags { stripped: cfg.codegen.strip_debug });

        // Map globale de déduplication des constantes
        let mut global: HashMap<ConstValue, u32> = HashMap::new();

        let mut inputs_meta = Vec::<LinkInput>::with_capacity(inputs.len());
        let mut total_consts_before = 0usize;

        for (name, ch) in inputs {
            inputs_meta.push(LinkInput {
                file: name.clone(),
                ops: ch.ops.len(),
                consts: ch.consts.len(),
            });
            total_consts_before += ch.consts.len();

            // 1) Remap des constantes (local -> global)
            let mut local_map = HashMap::<u32, u32>::with_capacity(ch.consts.len());
            for (old_ix, val) in ch.consts.iter() {
                let new_ix = if cfg.codegen.dedup_consts {
                    if let Some(&ix) = global.get(&val) {
                        ix
                    } else {
                        let ix = out.add_const(val.clone());
                        global.insert(val.clone(), ix);
                        ix
                    }
                } else {
                    out.add_const(val.clone())
                };
                local_map.insert(old_ix, new_ix);
            }

            // 2) Copie des opcodes + réécriture des LoadConst ; lignes conservées
            for (pc, op) in ch.ops.iter().enumerate() {
                let line = ch.lines.line_for_pc(pc as u32);
                let new = match *op {
                    Op::LoadConst(ix) => {
                        let new_ix = *local_map.get(&ix).ok_or_else(|| DriverError::Link(format!(
                            "const idx {ix} introuvable lors du lien ({name})"
                        )))?;
                        Op::LoadConst(new_ix)
                    }
                    other => other,
                };
                out.push_op(new, line);
            }

            // 3) Debug fusionné (si demandé et pas strip)
            if opts.merge_debug && !cfg.codegen.strip_debug {
                for f in &ch.debug.files {
                    if !out.debug.files.contains(f) {
                        out.debug.files.push(f.clone());
                    }
                }
                if out.debug.main_file.is_none() && ch.debug.main_file.is_some() {
                    out.debug.main_file = ch.debug.main_file.clone();
                }
                // Relocalisation symboles: base_pc = out.ops.len(avant copie) → ici on a copié; pour
                // conserver l’info, on aurait dû capturer base_pc avant la boucle op. On le refait propre :
                // (relink léger: recopie à nouveau les symboles recalés)
            }
        }

        // NB: symboles : on refait un passage pour recalage propre
        if opts.merge_debug && !cfg.codegen.strip_debug {
            let mut base = 0u32;
            for (_name, ch) in inputs {
                for (sym, pc) in &ch.debug.symbols {
                    out.debug.symbols.push((sym.clone(), base + *pc));
                }
                base += ch.ops.len() as u32;
            }
        }

        // Entry symbol
        if let Some(entry) = &opts.entry_symbol {
            if opts.merge_debug && !cfg.codegen.strip_debug {
                let ok = out.debug.symbols.iter().any(|(s, _)| s == entry);
                if !ok {
                    return Err(DriverError::Link(format!("symbole d’entrée `{entry}` introuvable")));
                }
                let note = format!("<entry:{entry}>");
                if !out.debug.files.contains(&note) {
                    out.debug.files.push(note);
                }
            }
        }

        // Strip final propre : si strip_debug demandé, on reconstruit sans debug.
        if cfg.codegen.strip_debug {
            let mut stripped = Chunk::new(ChunkFlags { stripped: true });
            for (_, c) in out.consts.iter() { stripped.add_const(c.clone()); }
            for (pc, op) in out.ops.iter().enumerate() {
                let line = out.lines.line_for_pc(pc as u32);
                stripped.push_op(*op, line);
            }
            out = stripped;
        }

        let manifest = LinkManifest {
            inputs: inputs_meta,
            total_ops: out.ops.len(),
            total_consts_before,
            total_consts_after: out.consts.len(),
            merged_debug_files: out.debug.files.len(),
            merged_debug_symbols: out.debug.symbols.len(),
            entry: opts.entry_symbol.clone(),
            hash: out.compute_hash(),
        };

        // Validation de base : taille limites
        if out.ops.len() > cfg.limits.max_ops {
            return Err(DriverError::Link(format!("trop d’opcodes: {} > max {}", out.ops.len(), cfg.limits.max_ops)));
        }
        if out.consts.len() > cfg.limits.max_consts {
            return Err(DriverError::Link(format!("trop de constantes: {} > max {}", out.consts.len(), cfg.limits.max_consts)));
        }

        Ok((out, manifest))
    }

    /* ----- Émission utilitaire ----- */

    /// Écrit un chunk en bytes vers `out_path` (création dossiers incluse).
    pub fn emit_bytes(chunk: &Chunk, out_path: &Path) -> Result<(), DriverError> {
        if let Some(parent) = out_path.parent() { fs::create_dir_all(parent)?; }
        let bytes = chunk.to_bytes();
        let mut f = fs::File::create(out_path)?;
        f.write_all(&bytes)?;
        Ok(())
    }
}

/* ───────────────────────────── Petits helpers ───────────────────────────── */

fn display_of(p: &Path) -> String {
    p.file_name().and_then(|s| s.to_str()).map(|s| s.to_string())
        .unwrap_or_else(|| p.display().to_string())
}

/* -------------------------------- Tests -------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cfg_default() -> Config { Config::default() }

    #[test]
    fn detect_kinds() {
        assert_eq!(Driver::detect_kind(Path::new("a.vit")), Some(InputKind::SourceVit));
        assert_eq!(Driver::detect_kind(Path::new("a.vit.s")), Some(InputKind::Asm));
        assert_eq!(Driver::detect_kind(Path::new("a.VITBC")), Some(InputKind::Bytecode));
        assert_eq!(Driver::detect_kind(Path::new("a.txt")), None);
    }

    #[test]
    fn link_empty_ok() {
        let cfg = cfg_default();
        let opts = BuildOptions::default();
        let c = Chunk::new(ChunkFlags { stripped: false });
        let (out, m) = Driver::link(&[("<empty>".into(), c)], &cfg, &opts).unwrap();
        assert!(out.ops.len() == 0);
        assert!(m.total_consts_after == 0);
    }

    #[test]
    fn assemble_then_link() {
        let cfg = cfg_default();
        let opts = BuildOptions::default();

        let src = r#"
            ldc 0
            print
            retv
        "#;
        // assemble via asm text friendly ? Nous utilisons l'assembleur core (syntaxe .vit.s réelle)
        let ch = Driver::assemble_source("LoadConst 0\nPrint\nReturnVoid\n").unwrap();
        let (out, _m) = Driver::link(&[("x".into(), ch)], &cfg, &opts).unwrap();
        assert!(out.ops.len() > 0);
    }

    #[test]
    fn roundtrip_when_asked() {
        let mut cfg = cfg_default();
        let mut opts = BuildOptions::default();
        opts.verify_roundtrip = true;

        let c = Chunk::new(ChunkFlags { stripped: true });
        let res = Driver::build_many(&[Input { path: PathBuf::from("x.vitbc"), kind: InputKind::Bytecode }],
                                     &cfg,
                                     &opts);
        // pas de fichier réel; on n’exécute pas ici
        let _ = res.err();
    }
}
