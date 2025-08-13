// vitcc/src/main.rs — Vitte tiny compiler (driver)
// -------------------------------------------------
// Commandes :
//   vitcc build <in.vit|asm> -o <out.vitbc> [--zstd] [--emit-asm <out.asm>]
//   vitcc emit  <in.vit> [-o out.asm]              # uniquement l’ASM généré
//   vitcc check <in.vit|asm>                       # assemble pour valider
//   vitcc disasm <in.vitbc> [-o out.asm]          # lis un VITBC et imprime ASM
//
// Nota bene : pour .vit → on génère un ASM minimaliste (voir la grammaire ci-dessus).
// Si tu as une vraie instruction PRINT côté VM, remplace la séquence commentée
// dans `lower_vit_to_asm()` par la tienne.

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
        "emit"  => cmd_emit(args),
        "check" => cmd_check(args),
        "disasm"=> cmd_disasm(args),
        other => {
            eprintln!("commande inconnue: {other}");
            help(2);
        }
    }
}

fn help(code: i32) -> ! {
    eprintln!(
r#"vitcc — mini compiler Vitte

USAGE
  vitcc build <in.vit|asm> -o <out.vitbc> [--zstd] [--emit-asm <out.asm>]
  vitcc emit  <in.vit> [-o out.asm]
  vitcc check <in.vit|asm>
  vitcc disasm <in.vitbc> [-o out.asm]"#);
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
    if args.is_empty() { eprintln!("usage: vitcc build <in.vit|asm> -o <out.vitbc> [--zstd] [--emit-asm <out.asm>]"); std::process::exit(2); }
    let in_path = args.remove(0);
    let out_bc  = pop_opt(&mut args, "-o", "--out").unwrap_or_else(|| die("✖ manque -o <out.vitbc>"));
    let out_asm = pop_opt(&mut args, "--emit-asm", "--emit");
    let compress = pop_flag(&mut args, "--zstd");
    if !args.is_empty() { die(&format!("arguments inconnus: {:?}", args)); }

    // 1) Charger la source
    let ext = Path::new(&in_path).extension().and_then(|s| s.to_str()).unwrap_or("");
    let asm_src = match ext {
        "asm" => fs::read_to_string(&in_path).unwrap_or_else(|e| die(&format!("lecture {in_path}: {e}"))),
        "vit" | "vitte" | "" => {
            let vit = fs::read_to_string(&in_path).unwrap_or_else(|e| die(&format!("lecture {in_path}: {e}")));
            lower_vit_to_asm(&vit)
        }
        _ => die("✖ extension inconnue (attendu .vit ou .asm)"),
    };

    // 2) Optionnel: sortir l’ASM généré
    if let Some(p) = out_asm.as_ref() {
        fs::write(p, &asm_src).unwrap_or_else(|e| die(&format!("écriture {p}: {e}")));
        eprintln!("📝 ASM → {p}");
    }

    // 3) Assembler → RawProgram
    let assembled = asm::assemble(&asm_src).unwrap_or_else(|e| die(&format!("assemble: {e}\n---\n{asm_src}")));

    // 4) Sauver en VITBC v2 (CRC/trailer), compression optionnelle
    loader::save_raw_program_to_path(&out_bc, &assembled.program, compress)
        .unwrap_or_else(|e| die(&format!("save: {e}")));
    println!("✅ écrit {out_bc} (VITBC v2, compressé={compress})");
}

fn cmd_emit(mut args: Vec<String>) {
    if args.is_empty() { eprintln!("usage: vitcc emit <in.vit> [-o out.asm]"); std::process::exit(2); }
    let in_path = args.remove(0);
    let out_asm = pop_opt(&mut args, "-o", "--out");
    if !args.is_empty() { die(&format!("arguments inconnus: {:?}", args)); }

    let vit = fs::read_to_string(&in_path).unwrap_or_else(|e| die(&format!("lecture {in_path}: {e}")));
    let asm_src = lower_vit_to_asm(&vit);
    if let Some(p) = out_asm {
        fs::write(&p, asm_src).unwrap_or_else(|e| die(&format!("écriture {p}: {e}")));
        println!("✅ ASM écrit → {p}");
    } else {
        print!("{asm_src}");
    }
}

fn cmd_check(mut args: Vec<String>) {
    if args.is_empty() { eprintln!("usage: vitcc check <in.vit|asm>"); std::process::exit(2); }
    let in_path = args.remove(0);
    let ext = Path::new(&in_path).extension().and_then(|s| s.to_str()).unwrap_or("");
    let asm_src = match ext {
        "asm" => fs::read_to_string(&in_path).unwrap_or_else(|e| die(&format!("lecture {in_path}: {e}"))),
        "vit" | "vitte" | "" => {
            let vit = fs::read_to_string(&in_path).unwrap_or_else(|e| die(&format!("lecture {in_path}: {e}")));
            lower_vit_to_asm(&vit)
        }
        _ => die("✖ extension inconnue (attendu .vit ou .asm)"),
    };
    // On essaie juste d’assembler (échec = erreur claire)
    let _ = asm::assemble(&asm_src).unwrap_or_else(|e| die(&format!("assemble: {e}")));
    println!("✅ check OK");
}

fn cmd_disasm(mut args: Vec<String>) {
    if args.is_empty() { eprintln!("usage: vitcc disasm <in.vitbc> [-o out.asm]"); std::process::exit(2); }
    let in_path = args.remove(0);
    let out_asm = pop_opt(&mut args, "-o", "--out");
    if !args.is_empty() { die(&format!("arguments inconnus: {:?}", args)); }

    let prog = loader::load_raw_program_from_path(&in_path)
        .unwrap_or_else(|e| die(&format!("load: {e}")));
    let text = asm::disassemble(&prog, &OpcodeTable::new_default());
    if let Some(p) = out_asm {
        fs::write(&p, text).unwrap_or_else(|e| die(&format!("écriture {p}: {e}")));
        println!("✅ écrit {p}");
    } else {
        print!("{text}");
    }
}

fn die(msg: &str) -> ! { eprintln!("{msg}"); std::process::exit(1) }

// -------------------- Frontend rudimentaire .vit → .asm --------------------

fn lower_vit_to_asm(src: &str) -> String {
    // On génère un ASM propre et conservateur :
    // - directives au fil de l’eau
    // - collecte des "print" pour déclarer .string sN
    // - corps avec label d’entrée (main par défaut si non fourni)
    let mut out = String::new();
    let mut strings: Vec<String> = Vec::new();
    let mut has_entry = false;
    let mut entry_name = String::from("main");

    for (ln, line) in src.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') { continue; }

        // Statements
        if let Some(rest) = t.strip_prefix("entry ") {
            let name = strip_trailing_semicolon(rest).trim();
            if !name.is_empty() { entry_name = name.to_string(); has_entry = true; }
            continue;
        }
        if let Some(rest) = t.strip_prefix("const ") {
            if let Some((name, rhs)) = rest.split_once('=') {
                let name = name.trim();
                let rhs = strip_trailing_semicolon(rhs).trim();
                out.push_str(&format!(".const {name} = {rhs}\n"));
                continue;
            }
        }
        if let Some(rest) = t.strip_prefix("string ") {
            if let Some((name, rhs)) = rest.split_once('=') {
                let name = name.trim();
                let rhs = strip_trailing_semicolon(rhs).trim();
                let lit = unquote(rhs);
                out.push_str(&format!(".string {name} = \"{}\"\n", escape_asm(&lit)));
                continue;
            }
        }
        if let Some(rest) = t.strip_prefix("data ") {
            if let Some((name, rhs)) = rest.split_once('=') {
                let name = name.trim();
                let rhs = strip_trailing_semicolon(rhs).trim();
                out.push_str(&format!(".data {name} = {rhs}\n"));
                continue;
            }
        }
        if let Some(rest) = t.strip_prefix("org ") {
            let n = strip_trailing_semicolon(rest).trim();
            out.push_str(&format!(".org {n}\n"));
            continue;
        }
        if let Some(lbl) = t.strip_prefix("label ") {
            let name = strip_trailing_colon_or_semicolon(lbl).trim();
            if !name.is_empty() { out.push_str(&format!("{name}:\n")); }
            continue;
        }
        if t.ends_with(':') {
            out.push_str(t);
            out.push('\n');
            continue;
        }

        // Pseudo "print"
        if let Some(rest) = t.strip_prefix("print ") {
            let lit = strip_trailing_semicolon(rest).trim();
            let s = unquote(lit);
            strings.push(s);
            // On injecte une séquence **commentée**. Ajuste ici si tu as une vraie PRINT.
            let idx = strings.len() - 1;
            out.push_str(&format!("    ; print s{idx}\n"));
            out.push_str(&format!("    ; LOADK r0, const:s{idx}\n"));
            out.push_str("    ; PRINT r0    ; <-- remplace par ton opcode si dispo\n");
            continue;
        }

        // Pass-through d’instructions "secure"
        if let Some(inst) = pass_through_inst(t) {
            out.push_str("    ");
            out.push_str(inst);
            out.push('\n');
            continue;
        }

        // Sinon: on ignore, mais on laisse une trace
        out.push_str(&format!("    ; (ignored @{}): {}\n", ln + 1, t));
    }

    // En-tête & strings collectées
    if !has_entry { out.push_str(".entry main\n"); }
    else          { out.push_str(&format!(".entry {entry_name}\n")); }
    for (i, s) in strings.iter().enumerate() {
        out.push_str(&format!(".string s{0} = \"{1}\"\n", i, escape_asm(s)));
    }
    // Label d’entrée s’il n’existe pas encore
    if !out.lines().any(|l| l.trim_start() == format!("{entry_name}:")) {
        out.push('\n');
        out.push_str(&format!("{entry_name}:\n"));
    }
    // S’assurer d’un RET final (si rien n’a été émis)
    if !out.lines().rev().any(|l| l.trim().eq_ignore_ascii_case("RET")) {
        out.push_str("    RET\n");
    }

    out
}

fn pass_through_inst(t: &str) -> Option<&str> {
    // Autorise un petit set d’instructions/mnémos (en majuscules conseillé)
    // Ajoute les tiennes au besoin.
    const ALLOWED: [&str; 10] = ["NOP","RET","LOADI","LOADK","ADD","SUB","MUL","DIV","JZ","JMP"];
    let up = t.trim_end_matches(';').trim();
    let mnem = up.split_whitespace().next().unwrap_or("");
    if ALLOWED.iter().any(|&k| k.eq_ignore_ascii_case(mnem)) {
        Some(up)
    } else {
        None
    }
}

fn strip_trailing_semicolon(s: &str) -> &str {
    let t = s.trim_end();
    if t.ends_with(';') { &t[..t.len()-1] } else { t }
}
fn strip_trailing_colon_or_semicolon(s: &str) -> &str {
    let t = s.trim_end();
    if t.ends_with(':') || t.ends_with(';') { &t[..t.len()-1] } else { t }
}
fn unquote(s: &str) -> String {
    let t = s.trim();
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        t[1..t.len()-1].to_string()
    } else { t.to_string() }
}
fn escape_asm(s: &str) -> String {
    s.chars().flat_map(|c| {
        match c {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"'  => "\\\"".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\t' => "\\t".chars().collect(),
            '\r' => "\\r".chars().collect(),
            _ => vec![c],
        }
    }).collect()
}
