// scripts/repl.rs
//
// REPL minimaliste pour Vitte.
// - Mode ASM (par défaut) : saisir/charger/assembler/désassembler/sauver un RawProgram.
// - Mode CHUNK (si vitte-core a la feature "eval") : construire un petit Chunk et l'exécuter.
//
// Ajoute ce binaire dans ton Cargo.toml (workspace ou crate utilitaire) :
// [[bin]]
// name = "vitte-repl"
// path = "scripts/repl.rs"
//
// Assure-toi que les deps sont dispo dans le workspace :
// vitte-vm   = { path = "vitte-vm", features = ["zstd"] }   # zstd optionnel, sinon retire-le
// vitte-core = { path = "vitte-core", features = ["eval"] } # pour :run en CHUNK mode
//
// Lance :
//   cargo run --bin vitte-repl
//
// Commandes (tape :help dans le REPL) :
//   :mode asm|chunk         — change de mode
//   :help                   — affiche l’aide
//   :clear                  — vide le buffer courant
//   :load <f.asm>           — charge l’ASM depuis un fichier (ASM mode)
//   :save <f.vitbc> [--zstd]— sauvegarde le bytecode VITBC (ASM mode, zstd si --zstd et feature active)
//   :assemble               — assemble l’ASM buffer → programme en mémoire (ASM mode)
//   :disasm                 — désassemble le programme courant (ASM mode)
//   :info                   — affiche des stats
//   :show                   — affiche le buffer ASM ou un résumé du chunk
//   :run                    — exécute le Chunk via vitte-core::runtime::eval (CHUNK mode)
//   :print "txt"            — ajoute Print("txt") dans le chunk (CHUNK mode)
//   :add a b                — ajoute a+b puis Print (CHUNK mode)
//   :bytes <out.vitbc>      — écrit le Chunk (format vitte-core, pas VITBC) (CHUNK mode)
//   :quit / :q              — quitte
//
// En ASM mode, toute ligne ne commençant pas par ':' est ajoutée au buffer ASM.

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use vitte_vm::{
    asm::{self, OpcodeTable, RawProgram},
    loader,
};

use vitte_core::{self, helpers as core_h, Op, ConstValue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode { Asm, Chunk }

struct State {
    mode: Mode,
    // ASM
    asm_buf: String,
    asm_prog: Option<RawProgram>,
    // CHUNK
    chunk: vitte_core::bytecode::chunk::Chunk,
}

impl State {
    fn new() -> Self {
        Self {
            mode: Mode::Asm,
            asm_buf: String::new(),
            asm_prog: None,
            chunk: core_h::new_chunk(false),
        }
    }
}

fn main() {
    println!("vitte-repl — bonjour ✨ (tape :help)");
    let mut st = State::new();
    let mut stdin = io::stdin();
    loop {
        print_prompt(st.mode);
        io::stdout().flush().ok();

        let mut line = String::new();
        if stdin.read_line(&mut line).is_err() { break; }
        if line.is_empty() { break; } // EOF
        let line = line.trim_end().to_string();
        if line.is_empty() { continue; }

        if line.starts_with(':') {
            if !handle_command(&mut st, &line[1..]) {
                break; // :quit
            }
        } else {
            // Ligne “contenu”
            match st.mode {
                Mode::Asm => {
                    st.asm_buf.push_str(&line);
                    st.asm_buf.push('\n');
                }
                Mode::Chunk => {
                    println!("(chunk) Ignoré : les actions chunk sont des commandes (:print, :add, :run, …)");
                }
            }
        }
    }
    println!("bye 👋");
}

fn print_prompt(mode: Mode) {
    match mode {
        Mode::Asm => print!("asm> "),
        Mode::Chunk => print!("chunk> "),
    }
}

// Retourne false pour quitter
fn handle_command(st: &mut State, cmdline: &str) -> bool {
    let mut parts = split_args(cmdline);
    let Some(cmd) = parts.get(0).cloned() else { return true; };

    match cmd.as_str() {
        "help" | "h" => {
            println!(
r#":mode asm|chunk     — change de mode
:help               — cette aide
:clear              — vide le buffer courant
:load <f.asm>       — charge l’ASM (ASM mode)
:save <f.vitbc> [--zstd]   — sauve en VITBC v2 (ASM mode)
:assemble           — assemble le buffer ASM → programme en mémoire
:disasm             — désassemble le programme courant
:info               — affiche des stats (consts, ops…)
:show               — montre le buffer ASM ou le chunk
:run                — exécute le chunk (feature eval requise)
:print \"txt\"        — ajoute Print(\"txt\") au chunk
:add <a> <b>        — ajoute a+b puis Print dans le chunk
:bytes <out.vitbc>  — écrit le Chunk (format vitte-core, pas VITBC)
:quit | :q          — quitter"#
            );
        }

        "mode" => {
            if let Some(m) = parts.get(1) {
                match m.as_str() {
                    "asm" => { st.mode = Mode::Asm; println!("→ mode ASM"); }
                    "chunk" => { st.mode = Mode::Chunk; println!("→ mode CHUNK"); }
                    other => println!("mode inconnu: {other} (asm|chunk)"),
                }
            } else {
                println!("usage: :mode asm|chunk");
            }
        }

        "clear" => {
            match st.mode {
                Mode::Asm => { st.asm_buf.clear(); st.asm_prog = None; println!("(asm) buffer vidé"); }
                Mode::Chunk => { st.chunk = core_h::new_chunk(false); println!("(chunk) réinitialisé"); }
            }
        }

        // ---------- ASM ----------
        "load" => {
            if st.mode != Mode::Asm { println!("(asm) uniquement"); return true; }
            if let Some(path) = parts.get(1) {
                match fs::read_to_string(path) {
                    Ok(s) => { st.asm_buf = s; st.asm_prog = None; println!("(asm) chargé {path}"); }
                    Err(e) => println!("✖ lecture {path}: {e}"),
                }
            } else {
                println!("usage: :load <f.asm>");
            }
        }
        "assemble" => {
            if st.mode != Mode::Asm { println!("(asm) uniquement"); return true; }
            match asm::assemble(&st.asm_buf) {
                Ok(out) => {
                    st.asm_prog = Some(out.program);
                    println!("(asm) ok — {} ops, {} consts, entry={:?}",
                        st.asm_prog.as_ref().unwrap().code.len(),
                        st.asm_prog.as_ref().unwrap().const_pool.ints.len()
                            + st.asm_prog.as_ref().unwrap().const_pool.floats.len()
                            + st.asm_prog.as_ref().unwrap().const_pool.strings.len(),
                        st.asm_prog.as_ref().unwrap().entry_pc);
                }
                Err(e) => println!("✖ assemble: {e}"),
            }
        }
        "disasm" => {
            if st.mode != Mode::Asm { println!("(asm) uniquement"); return true; }
            ensure_assembled(st);
            if let Some(ref prog) = st.asm_prog {
                let txt = asm::disassemble(prog, &OpcodeTable::new_default());
                println!("{txt}");
            }
        }
        "save" => {
            if st.mode != Mode::Asm { println!("(asm) uniquement"); return true; }
            let path = parts.get(1).cloned();
            if path.is_none() { println!("usage: :save <f.vitbc> [--zstd]"); return true; }
            let mut compress = false;
            if let Some(flag) = parts.get(2) {
                if flag == "--zstd" {
                    compress = true;
                }
            }
            ensure_assembled(st);
            if let Some(ref prog) = st.asm_prog {
                if let Err(e) = loader::save_raw_program_to_path(&path.unwrap(), prog, compress) {
                    println!("✖ save: {e}");
                } else {
                    println!("(asm) écrit {}", path.unwrap());
                }
            }
        }
        "info" => {
            match st.mode {
                Mode::Asm => {
                    ensure_assembled(st);
                    if let Some(ref p) = st.asm_prog {
                        println!("ASM program:");
                        println!("  entry_pc : {:?}", p.entry_pc);
                        println!("  code_ops : {}", p.code.len());
                        println!("  consts   : ints={} floats={} strings={}",
                            p.const_pool.ints.len(), p.const_pool.floats.len(), p.const_pool.strings.len());
                        println!("  data     : {}", p.data_blobs.len());
                    }
                }
                Mode::Chunk => {
                    let c = &st.chunk;
                    println!("Chunk:");
                    println!("  ops      : {}", c.ops.len());
                    println!("  consts   : {}", c.consts.len());
                    println!("  stripped : {}", c.flags.stripped);
                    println!("  main_file: {:?}", c.debug.main_file);
                }
            }
        }
        "show" => {
            match st.mode {
                Mode::Asm => {
                    if st.asm_buf.is_empty() { println!("(asm) <buffer vide>"); }
                    else { print!("{}", st.asm_buf); }
                }
                Mode::Chunk => {
                    println!("ops: {}", st.chunk.ops.len());
                    if st.chunk.ops.len() <= 64 {
                        println!("{:?}", st.chunk.ops);
                    } else {
                        println!("[…] ({} ops)", st.chunk.ops.len());
                    }
                }
            }
        }

        // ---------- CHUNK ----------
        "print" => {
            if st.mode != Mode::Chunk { println!("(chunk) uniquement"); return true; }
            let Some(arg1) = parts.get(1) else { println!("usage: :print \"message\""); return true; };
            let msg = unquote(arg1);
            let k = st.chunk.add_const(ConstValue::Str(msg.into()));
            st.chunk.ops.push(Op::LoadConst(k));
            st.chunk.ops.push(Op::Print);
            println!("(chunk) + Print");
        }
        "add" => {
            if st.mode != Mode::Chunk { println!("(chunk) uniquement"); return true; }
            let (a, b) = match (parts.get(1), parts.get(2)) {
                (Some(a), Some(b)) => (a.parse::<i64>(), b.parse::<i64>()),
                _ => { println!("usage: :add <i64> <i64>"); return true; }
            };
            if let (Ok(a), Ok(b)) = (a, b) {
                let ka = st.chunk.add_const(ConstValue::I64(a));
                let kb = st.chunk.add_const(ConstValue::I64(b));
                st.chunk.ops.push(Op::LoadConst(ka));
                st.chunk.ops.push(Op::LoadConst(kb));
                st.chunk.ops.push(Op::Add);
                st.chunk.ops.push(Op::Print);
                println!("(chunk) + Add/Print");
            } else {
                println!("valeurs invalides");
            }
        }
        "bytes" => {
            if st.mode != Mode::Chunk { println!("(chunk) uniquement"); return true; }
            let Some(path) = parts.get(1) else { println!("usage: :bytes <out.vitbc>"); return true; };
            let bytes = st.chunk.to_bytes();
            if let Err(e) = fs::write(path, bytes) {
                println!("✖ write {path}: {e}");
            } else {
                println!("(chunk) écrit {path} (format natif vitte-core)");
            }
        }
        "run" => {
            if st.mode != Mode::Chunk { println!("(chunk) uniquement"); return true; }
            // Nécessite vitte-core avec feature "eval"
            match run_chunk(&st.chunk) {
                Ok(out) => {
                    if !out.stdout.is_empty() { print!("{}", out.stdout); }
                    if !out.stderr.is_empty() { eprint!("{}", out.stderr); }
                }
                Err(e) => println!("✖ run: {e}"),
            }
        }

        // ---------- Quit ----------
        "quit" | "q" | "exit" => { return false; }

        // ---------- Inconnu ----------
        other => {
            println!("commande inconnue: :{other} (tape :help)");
        }
    }

    true
}

fn ensure_assembled(st: &mut State) {
    if st.asm_prog.is_some() { return; }
    match asm::assemble(&st.asm_buf) {
        Ok(out) => st.asm_prog = Some(out.program),
        Err(e) => println!("✖ assemble: {e}"),
    }
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        t[1..t.len()-1].to_string()
    } else {
        t.to_string()
    }
}

fn split_args(raw: &str) -> Vec<String> {
    // split naïf qui respecte des guillemets simples/doubles
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut q: Option<char> = None;
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match (q, c) {
            (None, '"') | (None, '\'') => { q = Some(c); }
            (Some(qc), ch) if ch == qc => { q = None; }
            (None, ' ' | '\t') => {
                if !cur.is_empty() { out.push(cur.clone()); cur.clear(); }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

// --------- Runner (CHUNK) ---------
// On délègue à vitte_core::runtime::eval (feature "eval" du crate vitte-core).
struct RunOut { stdout: String, stderr: String }

fn run_chunk(c: &vitte_core::bytecode::chunk::Chunk) -> Result<RunOut, String> {
    // Ces types proviennent de vitte-core::runtime::eval quand la feature "eval" est activée.
    // On compile en supposant que la feature est activée dans le Cargo.toml du binaire.
    use vitte_core::runtime::eval::{eval_chunk, EvalOptions};
    match eval_chunk(c, EvalOptions::default()) {
        Ok(out) => Ok(RunOut { stdout: out.stdout, stderr: out.stderr }),
        Err(e) => Err(format!("{e}")),
    }
}
