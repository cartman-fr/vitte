// crates/vitte-core/src/bin/vitbc-format.rs
//! Inspecteur du format .vitbc (rapport texte/JSON, hexdump, désasm).
//!
//! Exemples :
//!   cargo run -p vitte-core --bin vitbc-format -- a.vitbc --disasm=compact --verify
//!   cat a.vitbc | cargo run -p vitte-core --bin vitbc-format -- - --stdin-name a.vitbc --json
//!   cargo run -p vitte-core --bin vitbc-format -- a.vitbc b.vitbc --hexdump=256
//!
//! Options :
//!   --json                 Sortie JSON sur stdout
//!   --emit-json <fichier>  Écrit le JSON dans un fichier
//!   --summary              Résumé minimal (texte)
//!   --disasm=full|compact|none   Ajoute un désassemblage (def: none)
//!   --hexdump[=N]          Ajoute un hexdump (optionnellement limité à N octets)
//!   --stdin-name <nom>     Nom logique pour l’entrée '-'
//!   --verify               Round-trip (to_bytes -> from_bytes)
//!
//! NB: Aucune dépendance externe; parsing d’arguments minimaliste (std).

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use vitte_core::bytecode::chunk::Chunk as VChunk;
use vitte_core::disasm::{disassemble_compact, disassemble_full};

fn main() {
    if let Err(e) = real_main() {
        eprintln!("❌ {e}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), String> {
    let opts = Opts::parse(env::args().skip(1))?;

    if opts.inputs.is_empty() {
        return Err(usage());
    }

    for input in &opts.inputs {
        let (bytes, name) = read_input(input, &opts.stdin_name)?;
        process_one(&bytes, &name, &opts)?;
        if opts.inputs.len() > 1 {
            eprintln!("────────────────────────────────────────────────────────");
        }
    }

    Ok(())
}

/* ───────────────────────────── Arg parsing ───────────────────────────── */

#[derive(Clone, Copy, PartialEq, Eq)]
enum Disasm { None, Compact, Full }

struct Opts {
    inputs: Vec<String>,
    json: bool,
    emit_json: Option<PathBuf>,
    summary: bool,
    disasm: Disasm,
    hexdump: Option<usize>,
    stdin_name: String,
    verify: bool,
}

impl Opts {
    fn parse<I: Iterator<Item=String>>(mut args: I) -> Result<Self, String> {
        let mut o = Opts {
            inputs: vec![],
            json: false,
            emit_json: None,
            summary: false,
            disasm: Disasm::None,
            hexdump: None,
            stdin_name: "<stdin>".into(),
            verify: false,
        };

        while let Some(a) = args.next() {
            if a == "--json" {
                o.json = true;
            } else if a == "--summary" {
                o.summary = true;
            } else if a.starts_with("--disasm") {
                let v = if let Some(eq) = a.strip_prefix("--disasm=") {
                    eq.to_string()
                } else {
                    args.next().ok_or_else(|| "--disasm requiert une valeur".to_string())?
                };
                o.disasm = match v.as_str() {
                    "full" => Disasm::Full,
                    "compact" => Disasm::Compact,
                    "none" => Disasm::None,
                    _ => return Err("valeur invalide pour --disasm (full|compact|none)".into()),
                };
            } else if a.starts_with("--hexdump") {
                if let Some(eq) = a.strip_prefix("--hexdump=") {
                    let n = eq.parse::<usize>().map_err(|_| "--hexdump=N invalide".to_string())?;
                    o.hexdump = Some(n);
                } else {
                    o.hexdump = Some(usize::MAX);
                }
            } else if a == "--verify" {
                o.verify = true;
            } else if a == "--emit-json" {
                let p = args.next().ok_or_else(|| "--emit-json requiert un chemin".to_string())?;
                o.emit_json = Some(PathBuf::from(p));
            } else if let Some(v) = a.strip_prefix("--emit-json=") {
                o.emit_json = Some(PathBuf::from(v));
            } else if a == "--stdin-name" {
                o.stdin_name = args.next().ok_or_else(|| "--stdin-name requiert un nom".to_string())?;
            } else if let Some(v) = a.strip_prefix("--stdin-name=") {
                o.stdin_name = v.to_string();
            } else if a.starts_with("--") {
                return Err(format!("option inconnue: {a}\n{usage}", usage=usage()));
            } else {
                o.inputs.push(a);
            }
        }

        Ok(o)
    }
}

fn usage() -> String {
    let exe = "vitbc-format";
    format!(
"Usage:
  {exe} <f.vitbc|-> [options]

Options:
  --json                 Sortie JSON sur stdout
  --emit-json <fichier>  Écrit le JSON dans un fichier
  --summary              Résumé minimal (texte)
  --disasm=full|compact|none   Ajoute un désassemblage (def: none)
  --hexdump[=N]          Ajoute un hexdump (optionnellement limité à N octets)
  --stdin-name <nom>     Nom logique pour l’entrée '-'
  --verify               Round-trip (to_bytes -> from_bytes)
"
    )
}

/* ─────────────────────────── Traitement fichier ─────────────────────────── */

fn process_one(bytes: &[u8], name: &str, opts: &Opts) -> Result<(), String> {
    // Chargement & validation de forme (hash) via from_bytes
    let chunk = VChunk::from_bytes(bytes).map_err(|e| format!("Chargement échoué ({name}): {e}"))?;

    // Résumé ?
    if opts.summary && !opts.json {
        print_summary(&chunk, name);
    }

    // JSON ?
    if opts.json {
        let json = build_json(&chunk, name, opts);
        if let Some(path) = &opts.emit_json {
            fs::write(path, json.as_bytes()).map_err(|e| format!("écriture JSON: {e}"))?;
            eprintln!("🧾 JSON → {}", path.display());
        } else {
            println!("{json}");
        }
    } else {
        // Rapport texte
        print_text_report(&chunk, name, opts);
    }

    // Verify (round-trip)
    if opts.verify {
        let rt = chunk.to_bytes();
        let chk = VChunk::from_bytes(&rt).map_err(|e| format!("verify: round-trip échoué: {e}"))?;
        // hash constant si compute_hash() est pur
        let h1 = chunk.compute_hash();
        let h2 = chk.compute_hash();
        if h1 == h2 {
            eprintln!("✓ verify round-trip OK (hash=0x{h1:016x})");
        } else {
            return Err(format!("verify: hash différent après round-trip ({h1:016x} != {h2:016x})"));
        }
    }

    // Hexdump
    if let Some(limit) = opts.hexdump {
        hexdump(bytes, limit);
    }

    Ok(())
}

/* ──────────────────────────────── I/O ──────────────────────────────── */

fn read_input(arg: &str, stdin_name: &str) -> Result<(Vec<u8>, String), String> {
    if arg == "-" {
        let mut v = Vec::new();
        io::stdin().read_to_end(&mut v).map_err(|e| format!("lecture stdin: {e}"))?;
        Ok((v, stdin_name.to_string()))
    } else {
        let v = fs::read(arg).map_err(|e| format!("lecture {arg}: {e}"))?;
        Ok((v, arg.to_string()))
    }
}

/* ─────────────────────────── Impression texte ─────────────────────────── */

fn print_summary(chunk: &VChunk, name: &str) {
    let ops = chunk.ops.len();
    let consts = chunk.consts.len();
    let stripped = chunk.flags().stripped;
    let version = chunk.version();
    let hash = chunk.compute_hash();

    eprintln!("== {name} ==");
    eprintln!("• version: {}   stripped: {}   ops: {}   consts: {}   hash: 0x{hash:016x}",
              version, stripped, ops, consts);
}

fn print_text_report(chunk: &VChunk, name: &str, opts: &Opts) {
    // Header
    let version = chunk.version();
    let stripped = chunk.flags().stripped;
    let ops = chunk.ops.len();
    let consts = chunk.consts.len();
    let hash = chunk.compute_hash();
    println!("== {name} ==");
    println!("• version: {}   stripped: {}   ops: {}   consts: {}   hash: 0x{hash:016x}",
             version, stripped, ops, consts);

    // Const pool (aperçu)
    if consts > 0 {
        println!("\n# Const Pool");
        for (ix, c) in chunk.consts.iter().take(200) {
            let (ty, prev) = const_preview(c, 80);
            println!("  [{ix:03}] {:<5}  {}", ty, prev);
        }
        if consts > 200 { println!("  … ({} autres)", consts - 200); }
    } else {
        println!("\n# Const Pool (vide)");
    }

    // Line table
    println!("\n# Line Table (PC ranges)");
    let mut any = false;
    for (r, line) in chunk.lines.iter_ranges() {
        any = true;
        println!("  [{:05}..{:05})  line {}", r.start, r.end, line);
    }
    if !any { println!("  <aucune info de ligne>"); }

    // Debug
    println!("\n# Debug");
    match &chunk.debug.main_file {
        Some(m) => println!("  main_file: {m}"),
        None => println!("  main_file: (none)"),
    }
    if !chunk.debug.files.is_empty() {
        println!("  files ({}):", chunk.debug.files.len());
        for f in &chunk.debug.files { println!("    - {f}"); }
    } else {
        println!("  files: (none)");
    }
    if !chunk.debug.symbols.is_empty() {
        println!("  symbols ({}):", chunk.debug.symbols.len());
        for (sym, pc) in &chunk.debug.symbols {
            println!("    - {:05}  {}", pc, sym);
        }
    } else {
        println!("  symbols: (none)");
    }

    // Désassemblage (optionnel)
    match opts.disasm {
        Disasm::None => {}
        Disasm::Compact => {
            println!("\n# Code (compact)");
            print!("{}", disassemble_compact(chunk));
        }
        Disasm::Full => {
            println!();
            print!("{}", disassemble_full(chunk, name));
        }
    }
}

/* ─────────────────────────────── JSON out ─────────────────────────────── */

fn build_json(chunk: &VChunk, name: &str, opts: &Opts) -> String {
    // Stats const types
    let mut n_null=0usize; let mut n_bool=0usize; let mut n_i64=0usize; let mut n_f64=0usize;
    let mut n_str=0usize;  let mut n_bytes=0usize;
    for (_, c) in chunk.consts.iter() {
        match c {
            vitte_core::bytecode::chunk::ConstValue::Null => n_null+=1,
            vitte_core::bytecode::chunk::ConstValue::Bool(_) => n_bool+=1,
            vitte_core::bytecode::chunk::ConstValue::I64(_) => n_i64+=1,
            vitte_core::bytecode::chunk::ConstValue::F64(_) => n_f64+=1,
            vitte_core::bytecode::chunk::ConstValue::Str(_) => n_str+=1,
            vitte_core::bytecode::chunk::ConstValue::Bytes(_) => n_bytes+=1,
        }
    }

    // Build “à la main” (mini JSON safe)
    let mut s = String::new();
    s.push('{');
    push_kv_str(&mut s, "file", name);               s.push(',');
    push_kv_num(&mut s, "version", chunk.version()); s.push(',');
    push_kv_bool(&mut s, "stripped", chunk.flags().stripped); s.push(',');
    push_kv_num(&mut s, "ops", chunk.ops.len() as u64); s.push(',');
    push_kv_num(&mut s, "consts", chunk.consts.len() as u64); s.push(',');
    push_kv_hex64(&mut s, "hash", chunk.compute_hash()); s.push(',');

    // const type breakdown
    s.push_str("\"const_types\":{");
    push_kv_num(&mut s, "null", n_null as u64);   s.push(',');
    push_kv_num(&mut s, "bool", n_bool as u64);   s.push(',');
    push_kv_num(&mut s, "i64",  n_i64 as u64);    s.push(',');
    push_kv_num(&mut s, "f64",  n_f64 as u64);    s.push(',');
    push_kv_num(&mut s, "str",  n_str as u64);    s.push(',');
    push_kv_num(&mut s, "bytes",n_bytes as u64);
    s.push(')'.replace(')', '}')); // petite astuce pour clore correctement sans serde

    s.push(',');

    // lines (ranges)
    s.push_str("\"lines\":[");
    let mut first=true;
    for (r, line) in chunk.lines.iter_ranges() {
        if !first { s.push(','); }
        first=false;
        let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{{\"start\":{},\"end\":{},\"line\":{}}}", r.start, r.end, line));
    }
    s.push(']');

    s.push(',');
    // debug
    s.push_str("\"debug\":{");
    if let Some(m) = &chunk.debug.main_file {
        push_kv_str(&mut s, "main_file", m);
    } else {
        s.push_str("\"main_file\":null");
    }
    s.push(',');
    // files
    s.push_str("\"files\":[");
    for (i, f) in chunk.debug.files.iter().enumerate() {
        if i>0 { s.push(','); }
        push_json_str_raw(&mut s, f);
    }
    s.push(']');
    s.push(',');
    // symbols
    s.push_str("\"symbols\":[");
    for (i, (sym, pc)) in chunk.debug.symbols.iter().enumerate() {
        if i>0 { s.push(','); }
        let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{{\"pc\":{},\"name\":", pc));
        push_json_str_raw(&mut s, sym);
        s.push('}');
    }
    s.push('}');
    // options snapshot
    if opts.disasm != Disasm::None || opts.hexdump.is_some() || opts.verify || opts.summary {
        s.push(',');
        s.push_str("\"options\":{");
        push_kv_str(&mut s, "disasm", match opts.disasm { Disasm::None=>"none", Disasm::Compact=>"compact", Disasm::Full=>"full" });
        if let Some(n) = opts.hexdump { s.push(','); push_kv_num(&mut s, "hexdump_limit", n as u64); }
        if opts.verify { s.push(','); push_kv_bool(&mut s, "verify", true); }
        if opts.summary { s.push(','); push_kv_bool(&mut s, "summary", true); }
        s.push('}');
    }

    s.push('}');
    s
}

fn push_kv_str(dst: &mut String, key: &str, val: &str) {
    push_json_str_key(dst, key);
    dst.push(':');
    push_json_str_raw(dst, val);
}
fn push_kv_num(dst: &mut String, key: &str, val: u64) {
    push_json_str_key(dst, key);
    let _ = std::fmt::Write::write_fmt(dst, format_args!(":{val}"));
}
fn push_kv_bool(dst: &mut String, key: &str, val: bool) {
    push_json_str_key(dst, key);
    let _ = std::fmt::Write::write_fmt(dst, format_args!(":{}", if val { "true" } else { "false" }));
}
fn push_kv_hex64(dst: &mut String, key: &str, val: u64) {
    push_json_str_key(dst, key);
    let _ = std::fmt::Write::write_fmt(dst, format_args!(":\"0x{val:016x}\""));
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

/* ─────────────────────────────── Hexdump ─────────────────────────────── */

fn hexdump(bytes: &[u8], limit: usize) {
    println!("\n# Hexdump ({} octets{})", bytes.len(), if limit!=usize::MAX { format!(", limité à {limit}") } else { "".into() });
    let mut i = 0usize;
    let end = bytes.len().min(limit);
    while i < end {
        let line = &bytes[i .. end.min(i+16)];
        print!("{:08x}  ", i);
        for j in 0..16 {
            if j < line.len() { print!("{:02x} ", line[j]); }
            else { print!("   "); }
            if j == 7 { print!(" "); }
        }
        print!(" |");
        for &b in line {
            let c = if (32..=126).contains(&b) { b as char } else { '.' };
            print!("{c}");
        }
        println!("|");
        i += 16;
    }
}

/* ────────────────────────────── Petits helpers ───────────────────────────── */

fn const_preview(c: &vitte_core::bytecode::chunk::ConstValue, max: usize) -> (&'static str, String) {
    use vitte_core::bytecode::chunk::ConstValue::*;
    match c {
        Null => ("null", "null".into()),
        Bool(b) => ("bool", format!("{b}")),
        I64(i) => ("i64", format!("{i}")),
        F64(x) => ("f64", format!("{x}")),
        Str(s) => ("str", preview_str(s, max)),
        Bytes(b) => ("bytes", format!("len={}", b.len())),
    }
}

fn preview_str(s: &str, max: usize) -> String {
    let esc = escape_for_preview(s);
    if esc.len() <= max { format!("\"{esc}\"") } else { format!("\"{}…\"", &esc[..max]) }
}
fn escape_for_preview(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("\\x{:02X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}
