//! src/bin/vitte-links.rs — Linker multi-entrées Vitte ultra complet
//!
//! Exemples :
//!   vitte-links a.vitbc b.vit.s -o dist/app.vitbc --disasm full --json --map
//!   vitte-links main.vit lib.vit.s --stdlib prelude --pretty --entry main
//!   vitte-links - --stdin-kind asm --check --pretty
//!
//! Notes :
//! - Entrées supportées : .vit (si feature "frontend"), .vit.s, .vitbc
//! - '-' = stdin → préciser --stdin-kind (vit|asm|vitbc)
//! - Par défaut, écrit un unique .vitbc (link de toutes les entrées)
//! - Artefacts optionnels via `compiler::output` (disasm, asm, json, map, hex)
//! - Config lisible depuis l’env via `vitte_core::compiler::Config::from_env()`

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use camino::Utf8PathBuf;
use clap::{Parser, ValueEnum};

use vitte_core::compiler::config::{Config};
use vitte_core::compiler::driver::{BuildOptions, Driver, Input, InputKind};
use vitte_core::compiler::output::{EmitPlan, OutputKind, DisasmMode};
use vitte_core::pretty::{pretty_chunk_report, PrettyOptions};
use vitte_core::disasm::{disassemble_compact, disassemble_full};
use vitte_core::bytecode::chunk::Chunk;

#[derive(Copy, Clone, Debug, ValueEnum)]
enum StdlibMode { None, Prelude, All }

#[derive(Copy, Clone, Debug, ValueEnum)]
enum DisasmChoice { Full, Compact }

#[derive(Copy, Clone, Debug, ValueEnum)]
enum StdinKind { Vit, Asm, Vitbc }

#[derive(Parser, Debug)]
#[command(name="vitte-links", version, about="Linker (.vit/.vit.s/.vitbc -> .vitbc) + artefacts")]
struct Cli {
    /// Fichiers d’entrée (ou '-' pour stdin, avec --stdin-kind)
    inputs: Vec<String>,

    /// Chemin de sortie du bytecode final (.vitbc)
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Dossier de sortie commun pour les artefacts
    #[arg(long)]
    out_dir: Option<PathBuf>,

    /// Nom de base (sans extension) pour les artefacts
    #[arg(long)]
    stem: Option<String>,

    /// Mode stdlib: none | prelude | all  (nécessite feature `stdlib`)
    #[arg(long, value_enum, default_value_t=StdlibMode::None)]
    stdlib: StdlibMode,

    /// Symbole d’entrée à valider (présent dans le debug.symbols)
    #[arg(long)]
    entry: Option<String>,

    /// Retirer les infos de debug (strip)
    #[arg(long, default_value_t=false)]
    strip: bool,

    /// Désactiver la déduplication des constantes au lien
    #[arg(long, default_value_t=false)]
    no_dedup_consts: bool,

    /// Ne pas fusionner les infos de debug
    #[arg(long, default_value_t=false)]
    no_merge_debug: bool,

    /// Ne *pas* écrire de fichiers : assemble/compile + link + affiche selon flags
    #[arg(long, default_value_t=false)]
    check: bool,

    /// Désassemblage stdout du chunk linké (full|compact)
    #[arg(long, value_enum)]
    disasm: Option<DisasmChoice>,

    /// Rapport "pretty" (couleurs) stdout
    #[arg(long, default_value_t=false)]
    pretty: bool,

    /// Écrire aussi un pseudo-ASM .vit.s du chunk linké
    #[arg(long, default_value_t=false)]
    asm: bool,

    /// Écrire manifest JSON (.json)
    #[arg(long, default_value_t=false)]
    json: bool,

    /// Écrire source map minimal (.map.json)
    #[arg(long, default_value_t=false)]
    map: bool,

    /// Écrire hexdump (.hex.txt), option : limite d’octets
    #[arg(long = "hex")]
    hex_limit: Option<usize>,

    /// Titre pour les sorties textuelles (disasm/pretty). Sinon deduit.
    #[arg(long)]
    title: Option<String>,

    /// Vérifier to_bytes->from_bytes (sanity)
    #[arg(long, default_value_t=false)]
    verify_roundtrip: bool,

    /// PC de départ pour le listing pretty `--pretty`
    #[arg(long = "pc-start")]
    pc_start: Option<u32>,

    /// PC de fin (exclus) pour le listing pretty `--pretty`
    #[arg(long = "pc-end")]
    pc_end: Option<u32>,

    /// Résoudre le PC de départ à partir d’un symbole `--pretty`
    #[arg(long = "symbol")]
    symbol: Option<String>,

    /// Indique le type quand INPUT = '-' (vit|asm|vitbc)
    #[arg(long, value_enum)]
    stdin_kind: Option<StdinKind>,

    /// Limites (écrase celles venant de l’environnement)
    #[arg(long)]
    max_ops: Option<usize>,
    #[arg(long)]
    max_consts: Option<usize>,
}

fn main() {
    if let Err(e) = real_main() {
        eprintln!("❌ {e:#}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    color_eyre::install().ok();
    let cli = Cli::parse();

    if cli.inputs.is_empty() {
        bail!("aucun input. Fournis des fichiers .vit/.vit.s/.vitbc ou '-' (avec --stdin-kind).");
    }
    if cli.inputs.contains(&"-".into()) && cli.stdin_kind.is_none() {
        bail!("INPUT '-' détecté : précise --stdin-kind (vit|asm|vitbc).");
    }

    // 1) Config & options driver
    let mut cfg = Config::from_env();
    cfg.codegen.strip_debug = cli.strip;
    cfg.codegen.dedup_consts = !cli.no_dedup_consts;
    cfg.codegen.verify_roundtrip = cli.verify_roundtrip;
    if let Some(mo) = cli.max_ops { cfg.limits.max_ops = mo; }
    if let Some(mc) = cli.max_consts { cfg.limits.max_consts = mc; }

    let mut opts = BuildOptions::default();
    opts.merge_debug = !cli.no_merge_debug;
    opts.entry_symbol = cli.entry.clone();
    opts.verify_roundtrip = cli.verify_roundtrip;
    match cli.stdlib {
        StdlibMode::None => { opts.link_std = false; }
        StdlibMode::Prelude => { opts.link_std = true; opts.std_prelude_only = true; }
        StdlibMode::All => { opts.link_std = true; opts.std_prelude_only = false; }
    }

    // 2) Construire la liste d’inputs (avec support stdin)
    let inputs = make_inputs(&cli.inputs, cli.stdin_kind)?;

    // 3) Build & link
    let out = Driver::build_many(&inputs, &cfg, &opts)
        .context("échec build/link")?;

    let chunk = out.chunk;
    let mani = out.manifest;

    // 4) --check : imprime ce qu’on demande, ne rien écrire.
    if cli.check {
        eprintln!("🔗 Link OK: ops={} consts(before={} -> after={}) hash=0x{:016x}",
            mani.total_ops, mani.total_consts_before, mani.total_consts_after, mani.hash);
        eprintln!("   inputs: {}", mani.inputs.iter().map(|i| format!("{}(ops={},consts={})", i.file, i.ops, i.consts)).collect::<Vec<_>>().join(", "));
        if let Some(entry) = &mani.entry {
            eprintln!("   entry: {entry}");
        }

        if let Some(which) = cli.disasm {
            match which {
                DisasmChoice::Full => println!("{}", disassemble_full(&chunk, &title_for(&cli, &inputs))),
                DisasmChoice::Compact => println!("{}", disassemble_compact(&chunk)),
            }
        }
        if cli.pretty {
            let opts_pretty = PrettyOptions { color: true, ..Default::default() };
            // Si des bornes ont été fournies, on “filtre” en post en tronquant le listing
            let txt = pretty_custom(&chunk, &title_for(&cli, &inputs), &opts_pretty, cli.pc_start, cli.pc_end, cli.symbol.as_deref());
            println!("{txt}");
        }
        if cli.verify_roundtrip {
            verify_roundtrip(&chunk)?;
            eprintln!("🔁 Round-trip OK.");
        }
        return Ok(());
    }

    // 5) Émission de fichiers (bytecode obligatoire + artefacts optionnels)
    let mut plan = default_plan_for(&cli, &inputs)?;

    // Bytecode final
    let bc_out = cli.out.clone(); // Some(path) ou None -> résolu par plan
    plan = plan.with(OutputKind::Bytecode(bc_out));

    // Artefacts optionnels
    if let Some(which) = cli.disasm {
        let mode = match which { DisasmChoice::Full => DisasmMode::Full, DisasmChoice::Compact => DisasmMode::Compact };
        plan = plan.with(OutputKind::Disasm { mode, path: None });
    }
    if cli.asm { plan = plan.with(OutputKind::Asm { path: None }); }
    if cli.json { plan = plan.with(OutputKind::Json { path: None, pretty: true }); }
    if cli.map { plan = plan.with(OutputKind::SourceMap { path: None, pretty: true }); }
    if let Some(limit) = cli.hex_limit { plan = plan.with(OutputKind::Hexdump { path: None, limit: Some(limit) }); }

    // stem/out_dir overrides
    if let Some(stem) = &cli.stem { plan = plan.with_base_stem(stem.clone()); }
    if let Some(dir) = &cli.out_dir { plan = plan.with_out_dir(dir.clone()); }

    let arts = plan.emit_all(&chunk).context("écriture des artefacts")?;
    for a in &arts {
        eprintln!("📝 {} -> {}", a.kind, a.path.display());
    }
    if cli.verify_roundtrip {
        verify_roundtrip(&chunk)?;
        eprintln!("🔁 Round-trip OK.");
    }

    eprintln!("✅ Link fini. ops={} consts(before={} -> after={}) hash=0x{:016x}",
        mani.total_ops, mani.total_consts_before, mani.total_consts_after, mani.hash);
    Ok(())
}

/* ───────────────────────────── Impl détails ───────────────────────────── */

fn make_inputs(args: &[String], stdin_kind: Option<StdinKind>) -> Result<Vec<Input>> {
    let mut v = Vec::<Input>::with_capacity(args.len());
    for a in args {
        if a == "-" {
            // Lire tout stdin
            let mut s = String::new();
            io::stdin().read_to_string(&mut s)?;
            let kind = match stdin_kind.ok_or_else(|| anyhow!("--stdin-kind requis avec '-'"))? {
                StdinKind::Vit => InputKind::SourceVit,
                StdinKind::Asm => InputKind::Asm,
                StdinKind::Vitbc => {
                    // stdin bytes pas possible avec read_to_string → ré-ouvrir proprement
                    let mut b = Vec::<u8>::new();
                    io::stdin().read_to_end(&mut b)?;
                    // on ne peut pas reconstruire Input avec bytes brutes via Driver::build_many
                    // => imposons Vit/Vit.s sur stdin uniquement (simple et portable)
                    bail!("stdin pour .vitbc non supporté dans ce binaire (passe par fichier).");
                }
            };
            // On écrit la source dans un fichier temp si besoin ? Non : build_many attend des *paths*.
            // On fallback : crée un fichier temporaire pour stdin (simplicité).
            let tmp = write_temp_stdin(&s, match kind { InputKind::SourceVit => "stdin.vit", InputKind::Asm => "stdin.vit.s", InputKind::Bytecode => "stdin.vitbc" })?;
            v.push(Input { path: tmp, kind });
        } else {
            // Fichier normal : détecter l’extension via Driver::detect_kind
            let p = PathBuf::from(a);
            let kind = Driver::detect_kind(&p).ok_or_else(|| anyhow!("type d’entrée inconnu pour {}", a))?;
            v.push(Input { path: p, kind });
        }
    }
    Ok(v)
}

fn write_temp_stdin(content: &str, name: &str) -> Result<PathBuf> {
    let dir = std::env::temp_dir().join("vitte-links");
    fs::create_dir_all(&dir)?;
    let path = dir.join(name);
    let mut f = fs::File::create(&path)?;
    f.write_all(content.as_bytes())?;
    Ok(path)
}

fn default_plan_for(cli: &Cli, inputs: &[Input]) -> Result<EmitPlan> {
    // base: premier input
    if let Some(first) = inputs.first() {
        let p = &first.path;
        Ok(EmitPlan::for_input_path(p.clone()))
    } else {
        Ok(EmitPlan::new())
    }
}

fn title_for(cli: &Cli, inputs: &[Input]) -> String {
    if let Some(t) = &cli.title { return t.clone(); }
    if let Some(first) = inputs.first() {
        if let Some(name) = first.path.file_name().and_then(|s| s.to_str()) {
            return name.to_string();
        }
    }
    "chunk".into()
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

/// Version “custom” du pretty pour filtrer un range PC/symbole.
fn pretty_custom(chunk: &Chunk, title: &str, opts: &PrettyOptions, pc_start: Option<u32>, pc_end: Option<u32>, symbol: Option<&str>) -> String {
    use vitte_core::pretty::{compute_labels, pretty_chunk_header, pretty_const_pool_table, pretty_line_table, pretty_debug_info, pretty_op_line};
    let mut s = String::new();
    s.push_str(&pretty_chunk_header(chunk, title, opts)); s.push('\n');
    s.push_str(&pretty_const_pool_table(chunk, opts)); s.push('\n');
    s.push_str(&pretty_line_table(chunk, opts)); s.push('\n');
    s.push_str(&pretty_debug_info(chunk, opts)); s.push('\n');

    let mut start = pc_start.unwrap_or(0);
    if let Some(sym) = symbol {
        if let Some(pc) = chunk.debug.symbols.iter().find_map(|(s, pc)| if s == sym { Some(*pc) } else { None }) {
            start = pc;
        }
    }
    let end = pc_end.unwrap_or(chunk.ops.len() as u32);

    s.push_str("# Code\n");
    let labels = compute_labels(chunk);
    let from = start.min(chunk.ops.len() as u32);
    let to = end.min(chunk.ops.len() as u32);
    if from >= to {
        s.push_str("  (aucune instruction dans la plage demandée)\n");
    } else {
        for pc_usize in from as usize .. to as usize {
            let pc = pc_usize as u32;
            if let Some(lbl) = labels.get(&pc) {
                s.push_str(&format!("{}\n", lbl));
            }
            let line = chunk.lines.line_for_pc(pc);
            let row = pretty_op_line(chunk, pc, &chunk.ops[pc_usize], line, &labels, opts);
            s.push_str("  "); s.push_str(&row); s.push('\n');
        }
    }
    s
}

/* ───────────────────────────── Tests “fumants” ───────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;
    use vitte_core::bytecode::chunk::{ChunkFlags, ConstValue};
    use vitte_core::bytecode::op::Op;

    #[test]
    fn title_default_from_input() {
        let cli = Cli {
            inputs: vec!["a.vitbc".into()],
            out: None, out_dir: None, stem: None,
            stdlib: StdlibMode::None, entry: None, strip:false, no_dedup_consts:false,
            no_merge_debug:false, check:true, disasm:None, pretty:false,
            asm:false, json:false, map:false, hex_limit:None, title:None,
            verify_roundtrip:false, pc_start:None, pc_end:None, symbol:None,
            stdin_kind:None, max_ops:None, max_consts:None,
        };
        let title = title_for(&cli, &[
            Input { path: PathBuf::from("a.vitbc"), kind: InputKind::Bytecode }
        ]);
        assert_eq!(title, "a.vitbc");
    }

    #[test]
    fn verify_roundtrip_ok() {
        let mut c = Chunk::new(ChunkFlags { stripped: false });
        let k = c.add_const(ConstValue::I64(1));
        c.push_op(Op::LoadConst(k), Some(1));
        c.push_op(Op::ReturnVoid, Some(1));
        let _ = verify_roundtrip(&c).unwrap();
    }
}
