// tools/vitc/src/main.rs — "vitc": frontend/driver
//
// Commandes :
//   vitc build <in.(vit|asm)> -o <out.vitbc> [--zstd]
//   vitc check <in.(vit|asm)>
//   vitc disasm <in.vitbc> [-o out.asm]
//
// Politique pragmatique : si l'entrée est .asm → assemble directe.
// .vit : on tente une traduction naïve → ASM (placeholder) ou on refuse poliment.

use std::{fs, path::Path};

use vitte_vm::{
    asm::{self, OpcodeTable},
    loader,
};

fn main() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() { help(1); }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "help" | "-h" | "--help" => help(0),
        "build" => cmd_build(args),
        "check" => cmd_check(args),
        "disasm" => cmd_disasm(args),
        _ => { eprintln!("commande inconnue: {cmd}"); help(2); }
    }
}

fn help(code: i32) -> ! {
    eprintln!(
r#"vitc — Vitte Compiler (driver)

USAGE
  vitc build <in.(vit|asm)> -o <out.vitbc> [--zstd]
  vitc check <in.(vit|asm)>
  vitc disasm <in.vitbc> [-o out.asm]"#);
    std::process::exit(code)
}

fn pop_flag(args: &mut Vec<String>, flag: &str) -> bool {
    if let Some(i) = args.iter().position(|a| a == flag) { args.remove(i); true } else { false }
}
fn pop_opt(args: &mut Vec<String>, k1: &str, k2: &str) -> Option<String> {
    if let Some(i) = args.iter().position(|a| a == k1 || a == k2) {
        args.remove(i);
        if i < args.len() { Some(args.remove(i)) } else { None }
    } else { None }
}

fn cmd_build(mut args: Vec<String>) {
    if args.is_empty() { eprintln!("usage: vitc build <in.(vit|asm)> -o <out.vitbc> [--zstd]"); std::process::exit(2); }
    let in_path = args.remove(0);
    let out_path = pop_opt(&mut args, "-o", "--out").unwrap_or_else(|| {
        eprintln!("✖ manque -o <out.vitbc>"); std::process::exit(2)
    });
    let compress = pop_flag(&mut args, "--zstd");
    if !args.is_empty() {
        eprintln!("arguments inconnus: {:?}", args); std::process::exit(2);
    }

    let ext = Path::new(&in_path).extension().and_then(|s| s.to_str()).unwrap_or("");
    let asm_src = match ext {
        "asm" => fs::read_to_string(&in_path).unwrap_or_else(|e| die(&format!("lecture {in_path}: {e}"))),
        "vit" | "vitte" => lower_vit_to_asm(&fs::read_to_string(&in_path).unwrap_or_else(|e| die(&format!("lecture {in_path}: {e}")))),
        _ => { eprintln!("✖ extension inconnue (attendu .asm ou .vit)"); std::process::exit(2) }
    };

    let assembled = asm::assemble(&asm_src).unwrap_or_else(|e| die(&format!("assemble: {e}\n---\n{asm_src}")));
    loader::save_raw_program_to_path(&out_path, &assembled.program, compress)
        .unwrap_or_else(|e| die(&format!("save: {e}")));
    println!("✅ écrit {out_path} (VITBC v2, compressé={compress})");
}

fn cmd_check(mut args: Vec<String>) {
    if args.is_empty() { eprintln!("usage: vitc check <in.(vit|asm)>"); std::process::exit(2); }
    let in_path = args.remove(0);
    let ext = Path::new(&in_path).extension().and_then(|s| s.to_str()).unwrap_or("");
    let asm_src = match ext {
        "asm" => fs::read_to_string(&in_path).unwrap_or_else(|e| die(&format!("lecture {in_path}: {e}"))),
        "vit" | "vitte" => lower_vit_to_asm(&fs::read_to_string(&in_path).unwrap_or_else(|e| die(&format!("lecture {in_path}: {e}")))),
        _ => { eprintln!("✖ extension inconnue (attendu .asm ou .vit)"); std::process::exit(2) }
    };
    let _ = asm::assemble(&asm_src).unwrap_or_else(|e| die(&format!("assemble: {e}")));
    println!("✅ check OK");
}

fn cmd_disasm(mut args: Vec<String>) {
    if args.is_empty() { eprintln!("usage: vitc disasm <in.vitbc> [-o out.asm]"); std::process::exit(2); }
    let in_path = args.remove(0);
    let out_path = pop_opt(&mut args, "-o", "--out");
    let prog = loader::load_raw_program_from_path(&in_path)
        .unwrap_or_else(|e| die(&format!("load: {e}")));
    let text = asm::disassemble(&prog, &OpcodeTable::new_default());
    if let Some(o) = out_path {
        fs::write(&o, text).unwrap_or_else(|e| die(&format!("write {o}: {e}")));
        println!("✅ écrit {o}");
    } else {
        print!("{text}");
    }
}

// --- Frontend minimaliste : .vit → .asm (placeholder sensé)
//   Règles ridiculement simples :
//     print "txt"  -> génère .string + séquence LOADK/;CALL? dépend VM, alors on fallback à un label NOP + RET
//   Par défaut, on produit au moins un squelette valide.
fn lower_vit_to_asm(src: &str) -> String {
    let mut out = String::new();
    out.push_str("; vitc: frontend minimal .vit → .asm\n");
    out.push_str(".entry main\n");

    let mut strings: Vec<String> = Vec::new();
    for line in src.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") { continue; }
        if let Some(rest) = t.strip_prefix("print ") {
            // extrait "xxx" si présent
            let lit = rest.trim();
            let s = lit.trim_matches('"').to_string();
            strings.push(s);
        } else {
            // on ignore le reste pour l’instant
        }
    }
    // Constantes string
    for (i, s) in strings.iter().enumerate() {
        out.push_str(&format!(".string s{0} = \"{1}\"\n", i, escape_asm(s)));
    }
    // Corps minimal : NOP ; RET (si ta VM a PRINT, remplace ici par la séquence idoine)
    out.push_str("\nmain:\n");
    out.push_str("    NOP\n");
    out.push_str("    RET\n");
    out
}

fn escape_asm(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t")
}

fn die(msg: &str) -> ! {
    eprintln!("✖ {msg}");
    std::process::exit(1)
}
