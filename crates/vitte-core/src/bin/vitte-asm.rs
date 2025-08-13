//! src/bin/vitte-asm.rs — Assembleur Vitte 
//!
//! Exemples :
//!   vitte-asm foo.vit.s
//!   vitte-asm -o out/myprog.vitbc foo.vit.s --disasm full --json --map --hex 256
//!   cat foo.vit.s | vitte-asm - --check --pretty
//!
//! Notes :
//! - INPUT = '-' → lit depuis stdin
//! - Sans -o/--out, la sortie par défaut est `<stem>.vitbc` à côté de l’entrée
//! - `--strip` retire les infos de debug (et pose le flag stripped)
//! - `--check` n’écrit rien ; peut afficher `--disasm` / `--pretty` sur stdout
//! - Les artefacts fichiers utilisent `vitte_core::compiler::output::EmitPlan`

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, ValueEnum};

use vitte_core::compiler::output::{EmitPlan, OutputKind, DisasmMode};
use vitte_core::disasm::{disassemble_compact, disassemble_full};
use vitte_core::pretty::{pretty_chunk_report, PrettyOptions};
use vitte_core::bytecode::chunk::{Chunk, ChunkFlags, DebugInfo as ChunkDebug};
use vitte_core::bytecode::op::Op;

#[derive(Copy, Clone, Debug, ValueEnum)]
enum DisasmChoice { Full, Compact }

#[derive(Parser, Debug)]
#[command(name="vitte-asm", version, about="Assembleur Vitte (.vit.s -> .vitbc)")]
struct Cli {
    /// Fichier source .vit.s (ou '-' pour stdin)
    input: String,

    /// Fichier de sortie .vitbc. Par défaut : <input>.vitbc
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Dossier de sortie commun pour les artefacts (bytecode, json, etc.)
    #[arg(long)]
    out_dir: Option<PathBuf>,

    /// Nom de base à utiliser pour les artefacts (sans extension)
    #[arg(long)]
    stem: Option<String>,

    /// Retirer les infos de debug (strip)
    #[arg(long, default_value_t=false)]
    strip: bool,

    /// Vérifie l’assemblage sans écrire (dry-run)
    #[arg(long, default_value_t=false)]
    check: bool,

    /// Affiche un désassemblage sur stdout (full ou compact) — utile avec --check
    #[arg(long, value_enum)]
    disasm: Option<DisasmChoice>,

    /// Affiche un rapport joli/“pretty” (couleurs) sur stdout — utile avec --check
    #[arg(long, default_value_t=false)]
    pretty: bool,

    /// Écrit un pseudo-assembleur .vit.s (mnemonics) à côté du .vitbc (ou vers --stem/--out_dir)
    #[arg(long, default_value_t=false)]
    asm: bool,

    /// Écrit un manifest JSON (stats/hash/consts/lines/debug)
    #[arg(long, default_value_t=false)]
    json: bool,

    /// Écrit un source map minimal (.map.json)
    #[arg(long, default_value_t=false)]
    map: bool,

    /// Écrit un hexdump (.hex.txt). Optionnellement limite les octets loggés.
    #[arg(long = "hex")]
    hex_limit: Option<usize>,

    /// Titre à utiliser pour le désasm/pretty (sinon <stem>)
    #[arg(long)]
    title: Option<String>,

    /// Vérifie le round-trip to_bytes->from_bytes (sanity check)
    #[arg(long, default_value_t=false)]
    verify_roundtrip: bool,
}

fn main() {
    if let Err(err) = real_main() {
        eprintln!("❌ {err:#}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    color_eyre::install().ok();
    let cli = Cli::parse();

    // 1) Lire la source
    let (src, in_path_utf8, display_title) = read_source_and_title(&cli.input, cli.title.as_deref())?;

    // 2) Assembler
    let mut chunk = assemble(&src).context("Erreur d’assemblage")?;

    // 3) Injecter le nom de fichier dans le debug si dispo
    if in_path_utf8.as_deref() != Some("<stdin>") {
        let main = in_path_utf8.as_deref().unwrap().to_string();
        if chunk.debug.main_file.is_none() { chunk.debug.main_file = Some(main.clone()); }
        if !chunk.debug.files.contains(&main) { chunk.debug.files.push(main); }
    }

    // 4) Strip éventuel
    if cli.strip {
        chunk = strip_chunk(&chunk);
    }

    // 5) --check : rien n’est écrit ; on peut imprimer désasm/pretty
    if cli.check {
        eprintln!("✅ Assemblage OK (check-only). ops={}, consts={}, hash=0x{:016x}",
            chunk.ops.len(), chunk.consts.len(), chunk.compute_hash());

        if let Some(which) = cli.disasm {
            match which {
                DisasmChoice::Full => println!("{}", disassemble_full(&chunk, &display_title)),
                DisasmChoice::Compact => println!("{}", disassemble_compact(&chunk)),
            }
        }
        if cli.pretty {
            let opts = pretty_opts_auto();
            println!("{}", pretty_chunk_report(&chunk, &display_title, &opts));
        }
        if cli.verify_roundtrip {
            verify_roundtrip(&chunk)?;
            eprintln!("🔁 Round-trip OK.");
        }
        return Ok(());
    }

    // 6) Construction du plan d’émission fichiers
    let mut plan = plan_from_cli(&cli, in_path_utf8.clone(), cli.stem.clone());

    // Bytecode : toujours émis en mode non-check
    let bytecode_out = match &cli.out {
        Some(p) => Some(p.clone()),
        None => None, // laisser le plan résoudre vers <stem>.vitbc
    };
    plan = plan.with(OutputKind::Bytecode(bytecode_out));

    // Artefacts optionnels
    if let Some(which) = cli.disasm {
        let mode = match which {
            DisasmChoice::Full => DisasmMode::Full,
            DisasmChoice::Compact => DisasmMode::Compact,
        };
        plan = plan.with(OutputKind::Disasm { mode, path: None });
    }
    if cli.asm {
        plan = plan.with(OutputKind::Asm { path: None });
    }
    if cli.json {
        plan = plan.with(OutputKind::Json { path: None, pretty: true });
    }
    if cli.map {
        plan = plan.with(OutputKind::SourceMap { path: None, pretty: true });
    }
    if let Some(limit) = cli.hex_limit {
        plan = plan.with(OutputKind::Hexdump { path: None, limit: Some(limit) });
    }

    // Si l’utilisateur a demandé un stem explicite, on l’applique
    if let Some(stem) = &cli.stem {
        plan = plan.with_base_stem(stem.clone());
    }
    // Et un out_dir global éventuel
    if let Some(dir) = &cli.out_dir {
        plan = plan.with_out_dir(dir.clone());
    }

    // 7) Émission
    let artifacts = plan.emit_all(&chunk).context("Émission des artefacts")?;
    for a in &artifacts {
        eprintln!("📝 {} -> {}", a.kind, a.path.display());
    }

    // 8) Round-trip si demandé
    if cli.verify_roundtrip {
        verify_roundtrip(&chunk)?;
        eprintln!("🔁 Round-trip OK.");
    }

    eprintln!("✅ OK : ops={}, consts={}, hash=0x{:016x}",
        chunk.ops.len(), chunk.consts.len(), chunk.compute_hash());
    Ok(())
}

/* ───────────────────────────── Impl détails ───────────────────────────── */

fn read_source_and_title(input: &str, title_override: Option<&str>) -> Result<(String, Option<Utf8PathBuf>, String)> {
    if input == "-" {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s)?;
        let title = title_override
            .map(|t| t.to_string())
            .unwrap_or_else(|| "<stdin>".to_string());
        Ok((s, Some(Utf8PathBuf::from("<stdin>")), title))
    } else {
        let p = Utf8PathBuf::from_path_buf(PathBuf::from(input))
            .map_err(|_| anyhow!("Chemin non-UTF8: {input}"))?;
        let s = fs::read_to_string(&p)
            .with_context(|| format!("Lecture échouée: {p}"))?;
        let title = title_override.map(|t| t.to_string())
            .unwrap_or_else(|| p.file_name().unwrap_or("chunk").to_string());
        Ok((s, Some(p), title))
    }
}

fn assemble(src: &str) -> Result<Chunk> {
    vitte_core::asm::assemble(src).map_err(|e| anyhow!(e))
}

/// Reconstruit un chunk *strippé* (debug retiré + flag).
fn strip_chunk(orig: &Chunk) -> Chunk {
    let mut out = Chunk::new(ChunkFlags { stripped: true });
    // copier consts
    for (_ix, c) in orig.consts.iter() {
        out.add_const(c.clone());
    }
    // copier code + lignes
    for (pc, op) in orig.ops.iter().enumerate() {
        let line = orig.lines.line_for_pc(pc as u32);
        out.push_op(*op, line);
    }
    // debug minimal : vide (mais garde éventuellement un marqueur)
    out.debug = ChunkDebug::default();
    out
}

fn verify_roundtrip(chunk: &Chunk) -> Result<()> {
    let bytes = chunk.to_bytes();
    let chk = Chunk::from_bytes(&bytes).map_err(|e| anyhow!("from_bytes: {e}"))?;
    if chk.compute_hash() != chunk.compute_hash() {
        Err(anyhow!("hash différent après round-trip"))
    } else {
        Ok(())
    }
}

fn pretty_opts_auto() -> PrettyOptions {
    // Pas d’atty ici pour rester sans dépendances supplémentaires.
    PrettyOptions { color: true, ..Default::default() }
}

fn plan_from_cli(cli: &Cli, input_utf8: Option<Utf8PathBuf>, stem: Option<String>) -> EmitPlan {
    let mut plan = if let Some(p) = input_utf8.clone() {
        EmitPlan::for_input_path(p.as_std_path().to_path_buf())
    } else {
        EmitPlan::new()
    };
    if let Some(dir) = &cli.out_dir { plan = plan.with_out_dir(dir.clone()); }
    if let Some(stem) = stem { plan = plan.with_base_stem(stem); }
    plan
}

/* ───────────────────────────── (optionnel) helpers ASM quick ───────────────────────────── */
/* Ces fonctions ne sont pas indispensables mais utiles en dev local. */

#[allow(dead_code)]
fn demo_chunk_hello() -> Chunk {
    use vitte_core::bytecode::chunk::{ConstValue, ChunkFlags};
    let mut c = Chunk::new(ChunkFlags { stripped: false });
    let s = c.add_const(ConstValue::Str("Hello from vitte-asm".into()));
    c.push_op(Op::LoadConst(s), Some(1));
    c.push_op(Op::Print, Some(1));
    c.push_op(Op::ReturnVoid, Some(1));
    c
}

/* ───────────────────────────── Tests (si binaire testé) ───────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;
    use vitte_core::bytecode::chunk::{ChunkFlags, ConstValue};

    #[test]
    fn strip_preserves_code() {
        let mut c = Chunk::new(ChunkFlags{ stripped:false });
        let k = c.add_const(ConstValue::I64(1));
        c.push_op(Op::LoadConst(k), Some(1));
        c.push_op(Op::ReturnVoid, Some(1));
        let s = strip_chunk(&c);
        assert!(s.flags().stripped);
        assert_eq!(s.ops.len(), c.ops.len());
        assert_eq!(s.consts.len(), c.consts.len());
    }

    #[test]
    fn plan_defaults() {
        let cli = Cli {
            input: "foo.vit.s".into(),
            out: None, out_dir: None, stem: None, strip:false, check:false,
            disasm: None, pretty:false, asm:false, json:false, map:false,
            hex_limit: Some(64), title: None, verify_roundtrip:false,
        };
        let plan = plan_from_cli(&cli, Some(Utf8PathBuf::from("foo.vit.s")), None)
            .with(OutputKind::Bytecode(None))
            .with(OutputKind::Hexdump { path: None, limit: Some(64) });
        // pas d’IO ici, on s’assure que ça construit
        let _ = plan;
    }
}
