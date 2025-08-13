//! src/bin/vitte-disasm.rs — Désassembleur Vitte 
//!
//! Exemples rapides :
//!   vitte-disasm prog.vitbc
//!   vitte-disasm prog.vitbc --mode compact
//!   vitte-disasm prog.vitbc --mode pretty --no-consts --pc-start 10 --pc-end 200
//!   vitte-disasm a.vitbc b.vitbc --out-dir target/disasm --mode full --hex 256 --stats
//!   cat prog.vitbc | vitte-disasm - --mode pretty --search "hello"
//!
//! Notes :
//! - INPUT = '-' → lit le bytecode depuis stdin
//! - --out écrit TOUT dans un **fichier unique** (un seul input). --out-dir écrit 1 fichier par input.
//! - --mode pretty utilise `vitte_core::pretty` (couleurs ANSI contrôlées par --color)
//! - --pc-start/--pc-end filtrent le listing de code ; --symbol commence au PC du symbole
//! - --sections permet de choisir précisément ce qui s’affiche en mode pretty.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use camino::Utf8PathBuf;
use clap::{Parser, ValueEnum};

use vitte_core::bytecode::chunk::{Chunk, ConstValue};
use vitte_core::disasm::{disassemble_compact, disassemble_full};
use vitte_core::pretty::{
    PrettyOptions, pretty_chunk_report, pretty_code_listing, pretty_const_pool_table,
    pretty_chunk_header, pretty_debug_info, pretty_line_table, compute_labels, pretty_op_line,
    hexdump as pretty_hexdump,
};

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Mode { Full, Compact, Pretty }

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ColorChoice { Auto, Always, Never }

#[derive(Debug, Clone)]
struct Sections {
    header: bool,
    consts: bool,
    lines: bool,
    debug: bool,
    code: bool,
}

impl Sections {
    fn all() -> Self { Self { header: true, consts: true, lines: true, debug: true, code: true } }
    fn from_cli(no_header: bool, no_consts: bool, no_lines: bool, no_debug: bool, no_code: bool) -> Self {
        Self {
            header: !no_header,
            consts: !no_consts,
            lines: !no_lines,
            debug: !no_debug,
            code: !no_code,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name="vitte-disasm", version, about="Désassembleur Vitte (.vitbc -> texte)")]
struct Cli {
    /// Fichiers .vitbc à lire (ou '-' pour stdin)
    inputs: Vec<String>,

    /// Mode de sortie : full (core), compact (1 ligne/op), pretty (tableaux + couleurs)
    #[arg(long, value_enum, default_value_t=Mode::Full)]
    mode: Mode,

    /// Titre à afficher (sinon déduit du nom de fichier)
    #[arg(long)]
    title: Option<String>,

    /// Écrit la sortie vers ce fichier (seulement si 1 input)
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Écrit 1 fichier par input dans ce répertoire
    #[arg(long)]
    out_dir: Option<PathBuf>,

    /// Force le nom de base (sans extension) des fichiers générés (quand --out-dir)
    #[arg(long)]
    stem: Option<String>,

    /// Couleur ANSI : auto|always|never (uniquement pour --mode pretty)
    #[arg(long, value_enum, default_value_t=ColorChoice::Auto)]
    color: ColorChoice,

    /// N’afficher PAS l’entête
    #[arg(long, default_value_t=false)]
    no_header: bool,
    /// N’afficher PAS le pool de constantes
    #[arg(long, default_value_t=false)]
    no_consts: bool,
    /// N’afficher PAS la table des lignes
    #[arg(long, default_value_t=false)]
    no_lines: bool,
    /// N’afficher PAS les infos debug (fichiers/symboles)
    #[arg(long, default_value_t=false)]
    no_debug: bool,
    /// N’afficher PAS le listing de code
    #[arg(long, default_value_t=false)]
    no_code: bool,

    /// Filtrer le listing de code : PC de départ (inclus)
    #[arg(long = "pc-start")]
    pc_start: Option<u32>,
    /// Filtrer le listing de code : PC de fin (exclus)
    #[arg(long = "pc-end")]
    pc_end: Option<u32>,

    /// Positionner le PC de départ sur un symbole (ex: --symbol main)
    #[arg(long = "symbol")]
    symbol: Option<String>,

    /// Rechercher une chaîne (case sensitive) dans les constantes `Str` ; montre les hits
    #[arg(long)]
    search: Option<String>,

    /// Dump hexdump du binaire (optionnellement limite en octets)
    #[arg(long = "hex")]
    hex_limit: Option<usize>,

    /// Afficher une ligne de statistiques en fin (ops/consts/hash)
    #[arg(long, default_value_t=false)]
    stats: bool,

    /// Vérifie le round-trip to_bytes->from_bytes
    #[arg(long, default_value_t=false)]
    verify_roundtrip: bool,
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
        bail!("aucun input fourni. Utilise '-' pour stdin ou un .vitbc");
    }
    if cli.inputs.len() > 1 && cli.out.is_some() {
        bail!("--out n’est compatible qu’avec **un seul** input. Utilise --out-dir pour plusieurs fichiers.");
    }
    if cli.inputs.contains(&"-".to_string()) && cli.inputs.len() > 1 {
        bail!("stdin '-' ne peut pas être combiné avec d’autres entrées.");
    }

    let multi = cli.inputs.len() > 1;
    let sections = Sections::from_cli(cli.no_header, cli.no_consts, cli.no_lines, cli.no_debug, cli.no_code);

    for inp in &cli.inputs {
        let (bytes, stem, title) = read_input_and_title(inp, cli.title.as_deref())?;
        let chunk = load_chunk(&bytes).with_context(|| format!("échec de lecture {}", inp))?;

        if cli.verify_roundtrip {
            verify_roundtrip(&chunk)?;
        }

        // Optionnel : recherche dans const pool
        if let Some(pat) = &cli.search {
            print_search_hits(&chunk, pat);
        }

        // Contenu principal selon le mode
        let text = match cli.mode {
            Mode::Full => {
                // mode full n’est pas “sectionnable” — on pipe la version complétée
                let mut s = disassemble_full(&chunk, &title);
                if let Some(limit) = cli.hex_limit {
                    s.push_str("\n# Hexdump\n");
                    s.push_str(&pretty_hexdump(&bytes, limit));
                }
                if cli.stats {
                    s.push_str(&format!("\n# Stats: ops={} consts={} hash=0x{:016x}\n",
                        chunk.ops.len(), chunk.consts.len(), chunk.compute_hash()));
                }
                s
            }
            Mode::Compact => {
                let mut s = vitte_core::disasm::disassemble_compact(&chunk);
                if let Some(limit) = cli.hex_limit {
                    s.push_str("\n# Hexdump\n");
                    s.push_str(&pretty_hexdump(&bytes, limit));
                }
                if cli.stats {
                    s.push_str(&format!("\n# Stats: ops={} consts={} hash=0x{:016x}\n",
                        chunk.ops.len(), chunk.consts.len(), chunk.compute_hash()));
                }
                s
            }
            Mode::Pretty => {
                render_pretty(&chunk, &title, &sections, cli.pc_start, cli.pc_end, cli.symbol.as_deref(), color_enabled(cli.color), cli.hex_limit, cli.stats, &bytes)?
            }
        };

        // Sortie : stdout, --out (un seul), ou --out-dir
        if let Some(out_file) = &cli.out {
            write_file(out_file, text.as_bytes())?;
            eprintln!("📝 {} -> {}", stem.unwrap_or_else(|| "chunk".into()), out_file.display());
        } else if let Some(dir) = &cli.out_dir {
            let base = cli.stem.clone().unwrap_or_else(|| stem.unwrap_or_else(|| "chunk".into()));
            let fname = format!("{}.disasm.txt", base);
            let path = PathBuf::from(dir).join(fname);
            if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
            write_file(&path, text.as_bytes())?;
            eprintln!("📝 {} -> {}", base, path.display());
        } else {
            // Stdout + séparateur si multiple
            if multi {
                println!("===== [{}] =====", title);
            }
            print!("{text}");
            if !text.ends_with('\n') { println!(); }
        }
    }

    Ok(())
}

/* ───────────────────────────── Rendus & helpers ───────────────────────────── */

fn render_pretty(
    chunk: &Chunk,
    title: &str,
    sections: &Sections,
    pc_start: Option<u32>,
    pc_end: Option<u32>,
    symbol: Option<&str>,
    color: bool,
    hex_limit: Option<usize>,
    stats: bool,
    raw_bytes: &[u8],
) -> Result<String> {
    let mut out = String::new();
    let opts = PrettyOptions { color, ..Default::default() };

    // Résolution du PC de départ si symbole demandé
    let mut start = pc_start.unwrap_or(0);
    if let Some(sym) = symbol {
        if let Some(pc) = chunk.debug.symbols.iter().find_map(|(s, pc)| if s == sym { Some(*pc) } else { None }) {
            start = pc;
        } else {
            eprintln!("⚠️  symbole `{sym}` introuvable — on ignore.");
        }
    }
    let end = pc_end.unwrap_or(chunk.ops.len() as u32);

    if sections.header {
        out.push_str(&pretty_chunk_header(chunk, title, &opts));
        out.push('\n');
    }
    if sections.consts {
        out.push_str(&pretty_const_pool_table(chunk, &opts));
        out.push('\n');
    }
    if sections.lines {
        out.push_str(&pretty_line_table(chunk, &opts));
        out.push('\n');
    }
    if sections.debug {
        out.push_str(&pretty_debug_info(chunk, &opts));
        out.push('\n');
    }
    if sections.code {
        // Listing filtré par PC range
        out.push_str("# Code\n");
        let labels = compute_labels(chunk);
        let from = start.min(chunk.ops.len() as u32);
        let to = end.min(chunk.ops.len() as u32);
        if from >= to {
            out.push_str("  (aucune instruction dans la plage demandée)\n");
        } else {
            for pc_usize in from as usize .. to as usize {
                let pc = pc_usize as u32;
                if let Some(lbl) = labels.get(&pc) {
                    out.push_str(&format!("{}\n", lbl));
                }
                let line = chunk.lines.line_for_pc(pc);
                let row = pretty_op_line(chunk, pc, &chunk.ops[pc_usize], line, &labels, &opts);
                out.push_str("  "); out.push_str(&row); out.push('\n');
            }
        }
    }

    if let Some(limit) = hex_limit {
        out.push('\n');
        out.push_str("# Hexdump\n");
        out.push_str(&pretty_hexdump(raw_bytes, limit));
    }

    if stats {
        out.push_str(&format!("\n# Stats: ops={} consts={} hash=0x{:016x}\n",
            chunk.ops.len(), chunk.consts.len(), chunk.compute_hash()));
    }

    Ok(out)
}

fn read_input_and_title(input: &str, title_override: Option<&str>) -> Result<(Vec<u8>, Option<String>, String)> {
    if input == "-" {
        let mut buf = Vec::<u8>::new();
        io::stdin().read_to_end(&mut buf)?;
        let title = title_override.unwrap_or("<stdin>").to_string();
        Ok((buf, Some("chunk".into()), title))
    } else {
        let p = Utf8PathBuf::from_path_buf(PathBuf::from(input))
            .map_err(|_| anyhow!("Chemin non-UTF8: {input}"))?;
        let bytes = fs::read(&p)
            .with_context(|| format!("Lecture échouée: {p}"))?;
        let stem = p.file_stem().map(|s| s.to_string());
        let title = title_override.map(|t| t.to_string()).unwrap_or_else(|| p.file_name().unwrap_or("chunk").to_string());
        Ok((bytes, stem, title))
    }
}

fn load_chunk(bytes: &[u8]) -> Result<Chunk> {
    Chunk::from_bytes(bytes).map_err(|e| anyhow!(e))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let mut f = fs::File::create(path)?;
    f.write_all(bytes)?;
    Ok(())
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

fn color_enabled(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            // Heuristique simple sans dépendances: TERM présent et != "dumb"
            std::env::var("TERM").map(|t| t != "dumb").unwrap_or(false)
        }
    }
}

fn print_search_hits(chunk: &Chunk, pat: &str) {
    let mut found = 0usize;
    for (ix, c) in chunk.consts.iter() {
        if let ConstValue::Str(s) = c {
            if s.contains(pat) {
                println!("[const {:03}] match → {}", ix, s);
                found += 1;
            }
        }
    }
    if found == 0 {
        eprintln!("ℹ️  aucun match pour \"{pat}\" dans les constantes string.");
    }
}

/* ───────────────────────────── Tests “fumants” ───────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;
    use vitte_core::bytecode::chunk::{ChunkFlags};

    #[test]
    fn color_choice_auto() {
        assert!(matches!(color_enabled(ColorChoice::Always), true));
        assert!(matches!(color_enabled(ColorChoice::Never), false));
    }

    #[test]
    fn sections_mask() {
        let s = Sections::from_cli(false, true, false, true, false);
        assert!(s.header && !s.consts && s.lines && !s.debug && s.code);
    }

    #[test]
    fn roundtrip_ok() {
        // ne lit pas de fichier: juste assure la fonction compile
        let c = Chunk::new(ChunkFlags{ stripped:false });
        let _ = verify_roundtrip(&c);
    }
}
