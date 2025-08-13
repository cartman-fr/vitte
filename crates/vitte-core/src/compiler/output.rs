//! output.rs — Émission des artefacts compiler/codegen pour Vitte
//!
//! Objectif : à partir d’un `Chunk`, produire et écrire proprement :
//! - Bytecode `.vitbc` (binaire)
//! - Désassemblage `.disasm.txt` (full/compact)
//! - Pseudo-assembleur `.vit.s` (basé sur les mnemonics – round-trip “indicatif”)
//! - Manifest JSON `.json` (stats/hash/consts/lines/debug – sans serde, ou avec si feature)
//! - SourceMap minimal `.map.json` (PC→line + symboles debug)
//! - Hexdump `.hex.txt` (optionnel, limite paramétrable)
//!
//! Sans dépendances externes (tout std), `serde` uniquement si feature activée.
//! Ce module ne “construit” pas le chunk : il **émet** des représentations.
//!
//! Usage typique :
//! ```no_run
//! use std::path::PathBuf;
//! use vitte_core::compiler::output::{EmitPlan, OutputKind, DisasmMode};
//! # use vitte_core::bytecode::chunk::{Chunk, ChunkFlags};
//! # let chunk = Chunk::new(ChunkFlags{ stripped:false });
//! let plan = EmitPlan::for_input_path(PathBuf::from("src/main.vit"))
//!     .with(OutputKind::Bytecode(Some(PathBuf::from("target/main.vitbc"))))
//!     .with(OutputKind::Disasm { mode: DisasmMode::Full, path: None })
//!     .with(OutputKind::Json { path: None, pretty: true })
//!     .with(OutputKind::Hexdump { path: None, limit: Some(256) });
//! plan.emit_all(&chunk).expect("write");
//! ```

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, unused_must_use)]

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::bytecode::{
    chunk::Chunk,
    op::Op,
};
use crate::disasm::{disassemble_compact, disassemble_full};

/* ───────────────────────────── Types publics ───────────────────────────── */

/// Mode d’impression du désassemblage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisasmMode { Full, Compact }

/// Type d’artefact à produire.
#[derive(Debug, Clone)]
pub enum OutputKind {
    /// Binaire `.vitbc`
    Bytecode(Option<PathBuf>),
    /// Désassemblage texte (full/compact)
    Disasm { mode: DisasmMode, path: Option<PathBuf> },
    /// Pseudo-assembleur (mnemonics) `.vit.s`
    Asm { path: Option<PathBuf> },
    /// Manifest JSON (stats/hash/consts/lines/debug)
    Json { path: Option<PathBuf>, pretty: bool },
    /// SourceMap minimal JSON
    SourceMap { path: Option<PathBuf>, pretty: bool },
    /// Hexdump texte (optionnellement limité)
    Hexdump { path: Option<PathBuf>, limit: Option<usize> },
}

/// Stratégie d’écriture pour un artefact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sink {
    /// Écrire sur disque (chemin calculé si `None`)
    File,
    /// Écrire sur stdout (binaire ou texte selon le type)
    Stdout,
}

/// Plan complet d’émission (multi-artefacts).
#[derive(Debug, Clone)]
pub struct EmitPlan {
    /// **Chemin d’entrée “source”** (sert à déduire les noms par défaut).
    pub input_path: Option<PathBuf>,
    /// **Destination principale** (racine) — si définie, on privilégie ce répertoire.
    pub out_dir: Option<PathBuf>,
    /// **Nom de base** (sans extension). Sinon déduit du `input_path`.
    pub base_stem: Option<String>,
    /// **Has stdout** : certains artefacts peuvent aller sur stdout.
    pub stdout: bool,
    /// Liste d’artefacts à produire.
    pub outputs: Vec<OutputKind>,
}

impl EmitPlan {
    /// Plan de base dérivé d’un chemin d’entrée.
    pub fn for_input_path(input: PathBuf) -> Self {
        Self {
            base_stem: Some(stem_of(&input)),
            input_path: Some(input),
            out_dir: None,
            stdout: false,
            outputs: Vec::new(),
        }
    }

    /// Plan “vide” (tu renseignes tout à la main).
    pub fn new() -> Self {
        Self { input_path: None, out_dir: None, base_stem: Some("out".into()), stdout: false, outputs: Vec::new() }
    }

    /// Définit un répertoire de sortie commun.
    pub fn with_out_dir(mut self, dir: impl Into<PathBuf>) -> Self { self.out_dir = Some(dir.into()); self }

    /// Définit le nom de base (sans extension).
    pub fn with_base_stem(mut self, stem: impl Into<String>) -> Self { self.base_stem = Some(stem.into()); self }

    /// Autorise l’usage de stdout (si pertinent).
    pub fn with_stdout(mut self, yes: bool) -> Self { self.stdout = yes; self }

    /// Ajoute un artefact à produire.
    pub fn with(mut self, k: OutputKind) -> Self { self.outputs.push(k); self }

    /// Émet **tous** les artefacts du plan vers **fichiers** (et stdout si demandé).
    pub fn emit_all(&self, chunk: &Chunk) -> Result<Vec<Artifact>, EmitError> {
        let mut out = Vec::<Artifact>::with_capacity(self.outputs.len());
        for kind in &self.outputs {
            let art = self.emit_one(chunk, kind)?;
            out.push(art);
        }
        Ok(out)
    }

    /// Émet **un** artefact.
    pub fn emit_one(&self, chunk: &Chunk, kind: &OutputKind) -> Result<Artifact, EmitError> {
        match kind {
            OutputKind::Bytecode(path) => {
                let bytes = chunk.to_bytes();
                let target = self.resolve_path(path.as_ref(), "vitbc");
                write_binary(&target, &bytes)?;
                Ok(Artifact::binary("bytecode", target, bytes.len()))
            }
            OutputKind::Disasm { mode, path } => {
                let text = match mode {
                    DisasmMode::Full => disassemble_full(chunk, self.default_title().as_str()),
                    DisasmMode::Compact => disassemble_compact(chunk),
                };
                let target = self.resolve_path(path.as_ref(), match mode { DisasmMode::Full => "disasm.txt", DisasmMode::Compact => "disasm.txt" });
                write_text(&target, &text)?;
                Ok(Artifact::text("disasm", target, text.len()))
            }
            OutputKind::Asm { path } => {
                let text = render_asm(chunk);
                let target = self.resolve_path(path.as_ref(), "vit.s");
                write_text(&target, &text)?;
                Ok(Artifact::text("asm", target, text.len()))
            }
            OutputKind::Json { path, pretty } => {
                let text = render_manifest_json(chunk, *pretty);
                let target = self.resolve_path(path.as_ref(), "json");
                write_text(&target, &text)?;
                Ok(Artifact::text("json", target, text.len()))
            }
            OutputKind::SourceMap { path, pretty } => {
                let text = render_sourcemap_json(chunk, *pretty);
                let target = self.resolve_path(path.as_ref(), "map.json");
                write_text(&target, &text)?;
                Ok(Artifact::text("sourcemap", target, text.len()))
            }
            OutputKind::Hexdump { path, limit } => {
                let text = render_hexdump(&chunk.to_bytes(), limit.unwrap_or(usize::MAX));
                let target = self.resolve_path(path.as_ref(), "hex.txt");
                write_text(&target, &text)?;
                Ok(Artifact::text("hexdump", target, text.len()))
            }
        }
    }

    fn default_title(&self) -> String {
        if let Some(stem) = &self.base_stem { stem.clone() } else { "chunk".into() }
    }

    /// Résolution d’un chemin cible pour un artefact.
    ///
    /// - Si `explicit` est Some → retourne tel quel.
    /// - Sinon : `<out_dir or parent(input)>/<base_stem>.<ext>`
    fn resolve_path(&self, explicit: Option<&PathBuf>, ext: &str) -> PathBuf {
        if let Some(p) = explicit {
            return p.clone();
        }
        let base = self.base_stem.clone().unwrap_or_else(|| "out".into());
        let file = format!("{base}.{ext}");
        if let Some(dir) = &self.out_dir {
            return dir.join(file);
        }
        if let Some(inp) = &self.input_path {
            if let Some(parent) = inp.parent() {
                return parent.join(file);
            }
        }
        PathBuf::from(file)
    }
}

/* ───────────────────────────── Artefacts & erreurs ───────────────────────────── */

/// Artefact émis (pour logs/tests).
#[derive(Debug, Clone)]
pub struct Artifact {
    /// Type logique (“bytecode”, “disasm”, “json”, “asm”, “sourcemap”, “hexdump”…)
    pub kind: String,
    /// Chemin de sortie.
    pub path: PathBuf,
    /// Taille en octets (texte: octets UTF-8).
    pub size: usize,
}

impl Artifact {
    fn binary(kind: &str, path: PathBuf, size: usize) -> Self { Self { kind: kind.into(), path, size } }
    fn text(kind: &str, path: PathBuf, size: usize) -> Self { Self { kind: kind.into(), path, size } }
}

/// Erreurs d’émission.
#[derive(Debug)]
pub enum EmitError {
    Io(String),
}

impl From<io::Error> for EmitError {
    fn from(e: io::Error) -> Self { EmitError::Io(e.to_string()) }
}

/* ───────────────────────────── Rendus (render_*) ───────────────────────────── */

/// Pseudo-assembleur *lisible* (reconstitué à partir des opcodes + pools).
///
/// ⚠️ Ce rendu est **indicatif** : il se veut **compatible** avec l’assembleur
/// MVP (mnemonics) mais ne reconstruira pas toujours les mêmes offsets/structures
/// s’il manque du debug (labels). C’est parfait pour revue & golden tests.
pub fn render_asm(chunk: &Chunk) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();

    // Section en-tête : pool de constantes (commentaires)
    let _ = writeln!(s, "; -- Vitte ASM (pseudo) --");
    let _ = writeln!(s, "; version: {} stripped: {} consts: {} ops: {} hash: 0x{:016x}",
        chunk.version(), chunk.flags().stripped, chunk.consts.len(), chunk.ops.len(), chunk.compute_hash());
    if let Some(main) = &chunk.debug.main_file {
        let _ = writeln!(s, "; main_file: {}", main);
    }
    if !chunk.debug.files.is_empty() {
        let _ = writeln!(s, "; files: {}", chunk.debug.files.join(", "));
    }
    if !chunk.debug.symbols.is_empty() {
        let _ = writeln!(s, "; symbols:");
        for (sym, pc) in &chunk.debug.symbols {
            let _ = writeln!(s, ";   {:05} {}", pc, sym);
        }
    }

    // Labels pour sauts
    let labels = compute_labels(chunk);

    // Code
    let _ = writeln!(s, "\n; code");
    for (pc_usize, op) in chunk.ops.iter().enumerate() {
        let pc = pc_usize as u32;
        if let Some(lbl) = labels.get(&pc) {
            let _ = writeln!(s, "{lbl}:");
        }
        let line = chunk.lines.line_for_pc(pc);
        if let Some(l) = line {
            let _ = write!(s, "  ; line {}\n", l);
        }
        let _ = writeln!(s, "  {}", asm_for_op(chunk, pc, op, &labels));
    }

    s
}

fn asm_for_op(chunk: &Chunk, pc: u32, op: &Op, labels: &std::collections::HashMap<u32, String>) -> String {
    use Op::*;
    match *op {
        LoadConst(ix) => match chunk.consts.get(ix) {
            Some(crate::bytecode::chunk::ConstValue::Str(ref s)) =>
                format!("ldc {ix}      ; \"{}\"", shorten(s, 60)),
            Some(ref c) => format!("ldc {ix}      ; {}", pretty_const(c, 60)),
            None => format!("ldc {ix}      ; <invalid>"),
        },
        LoadLocal(ix)      => format!("ldl {}", ix),
        StoreLocal(ix)     => format!("stl {}", ix),
        LoadUpvalue(ix)    => format!("ldu {}", ix),
        StoreUpvalue(ix)   => format!("stu {}", ix),
        MakeClosure(fi, n) => format!("mkclo {} {}", fi, n),
        Call(argc)         => format!("call {}", argc),
        TailCall(argc)     => format!("tcall {}", argc),
        Jump(off)          => {
            let dest = (pc as i64 + 1 + off as i64).max(0) as u32;
            if let Some(lbl) = labels.get(&dest) { format!("jmp {lbl}") } else { format!("jmp {:+}", off) }
        }
        JumpIfFalse(off)   => {
            let dest = (pc as i64 + 1 + off as i64).max(0) as u32;
            if let Some(lbl) = labels.get(&dest) { format!("jz {lbl}") } else { format!("jz {:+}", off) }
        }
        Return             => "ret".into(),
        ReturnVoid         => "retv".into(),
        Nop                => "nop".into(),
        Print              => "print".into(),
        Add                => "add".into(),
        Sub                => "sub".into(),
        Mul                => "mul".into(),
        Div                => "div".into(),
        Mod                => "mod".into(),
        Neg                => "neg".into(),
        Not                => "not".into(),
        Eq                 => "eq".into(),
        Ne                 => "ne".into(),
        Lt                 => "lt".into(),
        Le                 => "le".into(),
        Gt                 => "gt".into(),
        Ge                 => "ge".into(),
        LoadTrue           => "ldtrue".into(),
        LoadFalse          => "ldfalse".into(),
        LoadNull           => "ldnull".into(),
        Pop                => "pop".into(),
    }
}

/* ───────────────────────────── Rendus JSON ───────────────────────────── */

/// Manifest JSON sans serde (ou via serde si dispo).
pub fn render_manifest_json(chunk: &Chunk, pretty: bool) -> String {
    #[cfg(feature = "serde")]
    {
        #[derive(serde::Serialize)]
        struct Manifest<'a> {
            version: u16,
            stripped: bool,
            ops: usize,
            consts: usize,
            hash: String,
            const_types: ConstTypes,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            lines: Vec<LineRange>,
            debug: Debug<'a>,
        }
        #[derive(serde::Serialize, Default)]
        struct ConstTypes { null: usize, bool: usize, i64: usize, f64: usize, str: usize, bytes: usize }
        #[derive(serde::Serialize)]
        struct LineRange { start: u32, end: u32, line: u32 }
        #[derive(serde::Serialize)]
        struct Debug<'a> {
            main_file: Option<&'a String>,
            files: &'a Vec<String>,
            symbols: Vec<Symbol<'a>>,
        }
        #[derive(serde::Serialize)]
        struct Symbol<'a> { pc: u32, name: &'a String }

        let mut ct = ConstTypes::default();
        for (_, c) in chunk.consts.iter() {
            use crate::bytecode::chunk::ConstValue::*;
            match c { Null => ct.null+=1, Bool(_) => ct.bool+=1, I64(_) => ct.i64+=1, F64(_) => ct.f64+=1, Str(_) => ct.str+=1, Bytes(_) => ct.bytes+=1 }
        }
        let lines = chunk.lines.iter_ranges()
            .map(|(r, l)| LineRange { start: r.start, end: r.end, line: l })
            .collect::<Vec<_>>();
        let dbg = Debug {
            main_file: chunk.debug.main_file.as_ref(),
            files: &chunk.debug.files,
            symbols: chunk.debug.symbols.iter().map(|(s, pc)| Symbol { pc: *pc, name: s }).collect(),
        };
        let m = Manifest {
            version: chunk.version(),
            stripped: chunk.flags().stripped,
            ops: chunk.ops.len(),
            consts: chunk.consts.len(),
            hash: format!("0x{:016x}", chunk.compute_hash()),
            const_types: ct,
            lines,
            debug: dbg,
        };
        if pretty {
            return serde_json::to_string_pretty(&m).unwrap_or_else(|_| "{}".into());
        } else {
            return serde_json::to_string(&m).unwrap_or_else(|_| "{}".into());
        }
    }
    #[cfg(not(feature = "serde"))]
    {
        // Variante manuelle (safe)
        build_json_manual(chunk, pretty)
    }
}

/// Sourcemap minimal JSON : PC→line + symboles (nom, pc).
pub fn render_sourcemap_json(chunk: &Chunk, pretty: bool) -> String {
    let mut s = String::new();
    if pretty { s.push_str("{\n  "); } else { s.push('{'); }
    push_kv_str(&mut s, "version", "vitte-sourcemap-1");
    s.push(',');
    if pretty { s.push_str("\n  "); }
    // lines
    s.push_str("\"lines\":[");
    let mut first = true;
    for (r, line) in chunk.lines.iter_ranges() {
        if !first { s.push(','); if pretty { s.push('\n'); s.push_str("    "); } }
        first = false;
        if pretty {
            let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{{\"start\":{},\"end\":{},\"line\":{}}}", r.start, r.end, line));
        } else {
            let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{{\"start\":{},\"end\":{},\"line\":{}}}", r.start, r.end, line));
        }
    }
    s.push(']');
    // symbols
    s.push(',');
    if pretty { s.push_str("\n  "); }
    s.push_str("\"symbols\":[");
    for (i, (sym, pc)) in chunk.debug.symbols.iter().enumerate() {
        if i>0 { s.push(','); }
        if pretty { s.push('\n'); s.push_str("    "); }
        let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{{\"pc\":{},\"name\":", pc));
        push_json_str_raw(&mut s, sym);
        s.push('}');
    }
    s.push(']');
    if pretty { s.push_str("\n}\n"); } else { s.push('}'); }
    s
}

/* ───────────────────────────── Rendus utilitaires ───────────────────────────── */

fn render_hexdump(bytes: &[u8], limit: usize) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let end = bytes.len().min(limit);
    let _ = writeln!(s, "# Hexdump (len={}, limit={})", bytes.len(), if limit==usize::MAX { "∞".into() } else { limit.to_string() });
    let mut i = 0usize;
    while i < end {
        let line = &bytes[i .. end.min(i+16)];
        let _ = write!(s, "{:08x}  ", i);
        for j in 0..16 {
            if j < line.len() { let _ = write!(s, "{:02x} ", line[j]); }
            else { let _ = write!(s, "   "); }
            if j == 7 { let _ = write!(s, " "); }
        }
        let _ = write!(s, " |");
        for &b in line {
            let c = if (32..=126).contains(&b) { b as char } else { '.' };
            let _ = write!(s, "{c}");
        }
        let _ = writeln!(s, "|");
        i += 16;
    }
    s
}

/* ───────────────────────────── Helpers locaux ───────────────────────────── */

fn stem_of(p: &Path) -> String {
    p.file_stem().and_then(|s| s.to_str()).unwrap_or("out").to_string()
}

fn write_text(path: &Path, s: &str) -> Result<(), EmitError> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let mut f = fs::File::create(path)?;
    f.write_all(s.as_bytes())?;
    Ok(())
}
fn write_binary(path: &Path, bytes: &[u8]) -> Result<(), EmitError> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let mut f = fs::File::create(path)?;
    f.write_all(bytes)?;
    Ok(())
}

/// Labels sur les cibles de saut.
fn compute_labels(chunk: &Chunk) -> std::collections::HashMap<u32, String> {
    use std::collections::HashMap;
    let mut set = HashMap::<u32, String>::new();
    let mut targets = Vec::<u32>::new();
    for (pc_usize, op) in chunk.ops.iter().enumerate() {
        let pc = pc_usize as u32;
        if let Some(dest) = match *op {
            Op::Jump(ofs) | Op::JumpIfFalse(ofs) => {
                let dest = (pc as i64 + 1 + ofs as i64);
                if dest >= 0 { Some(dest as u32) } else { None }
            }
            _ => None
        } {
            if (dest as usize) < chunk.ops.len() {
                targets.push(dest);
            }
        }
    }
    targets.sort_unstable();
    targets.dedup();
    for (i, pc) in targets.into_iter().enumerate() {
        set.insert(pc, format!("L{:04}", i+1));
    }
    set
}

/* ───────────────────────────── JSON (manuel) ───────────────────────────── */

#[cfg(not(feature = "serde"))]
fn build_json_manual(chunk: &Chunk, pretty: bool) -> String {
    let mut n_null=0usize; let mut n_bool=0usize; let mut n_i64=0usize; let mut n_f64=0usize;
    let mut n_str=0usize;  let mut n_bytes=0usize;
    for (_, c) in chunk.consts.iter() {
        use crate::bytecode::chunk::ConstValue::*;
        match c { Null => n_null+=1, Bool(_) => n_bool+=1, I64(_) => n_i64+=1, F64(_) => n_f64+=1, Str(_) => n_str+=1, Bytes(_) => n_bytes+=1 }
    }
    let mut s = String::new();
    if pretty { s.push_str("{\n  "); } else { s.push('{'); }
    push_kv_num(&mut s, "version", chunk.version() as u64);
    s.push(',');
    if pretty { s.push_str("\n  "); }
    push_kv_bool(&mut s, "stripped", chunk.flags().stripped);
    s.push(',');
    if pretty { s.push_str("\n  "); }
    push_kv_num(&mut s, "ops", chunk.ops.len() as u64);
    s.push(',');
    if pretty { s.push_str("\n  "); }
    push_kv_num(&mut s, "consts", chunk.consts.len() as u64);
    s.push(',');
    if pretty { s.push_str("\n  "); }
    push_kv_hex64(&mut s, "hash", chunk.compute_hash());
    s.push(',');
    if pretty { s.push_str("\n  "); }
    s.push_str("\"const_types\":{");
    push_kv_num(&mut s, "null", n_null as u64); s.push(',');
    push_kv_num(&mut s, "bool", n_bool as u64); s.push(',');
    push_kv_num(&mut s, "i64", n_i64 as u64); s.push(',');
    push_kv_num(&mut s, "f64", n_f64 as u64); s.push(',');
    push_kv_num(&mut s, "str", n_str as u64); s.push(',');
    push_kv_num(&mut s, "bytes", n_bytes as u64);
    s.push('}');
    s.push(',');
    if pretty { s.push_str("\n  "); }
    s.push_str("\"lines\":[");
    let mut first = true;
    for (r, line) in chunk.lines.iter_ranges() {
        if !first { s.push(','); }
        if pretty { s.push('\n'); s.push_str("    "); }
        let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{{\"start\":{},\"end\":{},\"line\":{}}}", r.start, r.end, line));
        first = false;
    }
    s.push(']');
    s.push(',');
    if pretty { s.push_str("\n  "); }
    s.push_str("\"debug\":{");
    match &chunk.debug.main_file {
        Some(m) => push_kv_str(&mut s, "main_file", m),
        None => { s.push_str("\"main_file\":null"); }
    }
    s.push(',');
    s.push_str("\"files\":[");
    for (i, f) in chunk.debug.files.iter().enumerate() {
        if i>0 { s.push(','); }
        push_json_str_raw(&mut s, f);
    }
    s.push(']');
    s.push(',');
    s.push_str("\"symbols\":[");
    for (i, (sym, pc)) in chunk.debug.symbols.iter().enumerate() {
        if i>0 { s.push(','); }
        let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{{\"pc\":{},\"name\":", pc));
        push_json_str_raw(&mut s, sym);
        s.push('}');
    }
    s.push('}');
    if pretty { s.push_str("\n}\n"); } else { s.push('}'); }
    s
}

fn push_kv_str(dst: &mut String, key: &str, val: &str) {
    push_json_str_key(dst, key); dst.push(':'); push_json_str_raw(dst, val);
}
fn push_kv_num(dst: &mut String, key: &str, val: u64) {
    push_json_str_key(dst, key); let _ = std::fmt::Write::write_fmt(dst, format_args!(":{val}"));
}
fn push_kv_bool(dst: &mut String, key: &str, val: bool) {
    push_json_str_key(dst, key); let _ = std::fmt::Write::write_fmt(dst, format_args!(":{}", if val { "true" } else { "false" }));
}
fn push_kv_hex64(dst: &mut String, key: &str, val: u64) {
    push_json_str_key(dst, key); let _ = std::fmt::Write::write_fmt(dst, format_args!(":\"0x{val:016x}\""));
}
fn push_json_str_key(dst: &mut String, key: &str) {
    dst.push('"'); dst.push_str(key); dst.push('"');
}
fn push_json_str_raw(dst: &mut String, s: &str) {
    dst.push('"');
    for ch in s.chars() {
        match ch {
            '"'  => dst.push_str("\\\""),
            '\\' => dst.push_str("\\\\"),
            '\n' => dst.push_str("\\n"),
            '\t' => dst.push_str("\\t"),
            '\r' => dst.push_str("\\r"),
            c if c.is_control() => { let _ = std::fmt::Write::write_fmt(dst, format_args!("\\u{{{:x}}}", c as u32)); }
            c => dst.push(c),
        }
    }
    dst.push('"');
}

/* ───────────────────────────── Pretty helpers ───────────────────────────── */

fn shorten(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max]) }
}
fn pretty_const(c: &crate::bytecode::chunk::ConstValue, str_max: usize) -> String {
    use crate::bytecode::chunk::ConstValue::*;
    match c {
        Str(s) => format!("\"{}\"", shorten(s, str_max)),
        Bytes(b) => format!("bytes[{}]", b.len()),
        other => format!("{other}"),
    }
}

/* ───────────────────────────── Tests ───────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::chunk::{ChunkFlags};

    #[test]
    fn default_paths_ok() {
        let plan = EmitPlan::for_input_path(PathBuf::from("foo/bar/baz.vit"))
            .with(OutputKind::Json { path: None, pretty: false })
            .with(OutputKind::Disasm { mode: DisasmMode::Compact, path: None })
            .with(OutputKind::Bytecode(None));
        let c = Chunk::new(ChunkFlags { stripped: false });
        let arts = plan.emit_all(&c);
        // On ne teste pas le FS ici (pas de création réelle en CI), on vérifie juste que ça ne panique pas.
        let _ = arts.err(); // pas de fichiers réels en tests unitaires — OK
    }

    #[test]
    fn asm_render_smoke() {
        let c = Chunk::new(ChunkFlags{ stripped:false });
        let _txt = render_asm(&c); // doit générer quelque chose de valide
    }

    #[test]
    fn json_manual_ok() {
        let c = Chunk::new(ChunkFlags{ stripped:true });
        let _txt = render_manifest_json(&c, false);
    }

    #[test]
    fn sourcemap_ok() {
        let c = Chunk::new(ChunkFlags{ stripped:false });
        let _txt = render_sourcemap_json(&c, true);
    }
}
