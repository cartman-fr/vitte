// scripts/disasm.rs
//
// Mini outil CLI : désassemble un fichier VITBC en texte lisible.
//
// Usage :
//   cargo run --bin disasm -- <input.vitbc>
//   cargo run --bin disasm -- -                  # lit depuis stdin
//   cargo run --bin disasm -- input.vitbc -o out.asm
//   cargo run --bin disasm -- input.vitbc --stats --dump-data --max-bytes 256
//
// Remarques :
// - S’appuie sur vitte_vm::{loader, asm} (ton crate).
// - N’utilise PAS de dépendances externes (pas de clap & co).
// - --dump-data fait un hex dump lisible des blobs .data (adresse si connue).

use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use vitte_vm::{
    asm::{self, OpcodeTable},
    loader,
};

#[derive(Default)]
struct Opts {
    input: Option<String>,
    out: Option<String>,
    stats: bool,
    dump_data: bool,
    max_bytes: usize, // pour hex dump
    width: usize,     // octets par ligne
}

fn print_help() {
    eprintln!(
r#"disasm — désassemble un VITBC vers ASM lisible

USAGE:
  disasm <input.vitbc> [options]
  disasm -              # lit depuis stdin

Options:
  -o, --out <PATH>      Fichier de sortie (sinon stdout)
      --stats           Affiche un en-tête de stats (counts, entry_pc)
      --dump-data       Hex dump des blobs .data
      --max-bytes <N>   Limite d’octets à dumper par blob (défaut: 512)
      --width <N>       Octets par ligne dans le dump (défaut: 16)
  -h, --help            Affiche cette aide"#
    );
}

fn parse_args() -> Opts {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_help();
        std::process::exit(1);
    }
    let mut o = Opts { max_bytes: 512, width: 16, ..Default::default() };

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-o" | "--out" => {
                i += 1;
                if i >= args.len() { eprintln!("--out requiert un chemin"); std::process::exit(2); }
                o.out = Some(args[i].clone());
            }
            "--stats" => o.stats = true,
            "--dump-data" => o.dump_data = true,
            "--max-bytes" => {
                i += 1;
                if i >= args.len() { eprintln!("--max-bytes requiert une valeur"); std::process::exit(2); }
                o.max_bytes = args[i].parse().unwrap_or_else(|_| { eprintln!("--max-bytes invalide"); std::process::exit(2) });
            }
            "--width" => {
                i += 1;
                if i >= args.len() { eprintln!("--width requiert une valeur"); std::process::exit(2); }
                o.width = args[i].parse().unwrap_or_else(|_| { eprintln!("--width invalide"); std::process::exit(2) });
                if o.width == 0 { eprintln!("--width doit être > 0"); std::process::exit(2); }
            }
            s if s.starts_with('-') && s != "-" => {
                eprintln!("Option inconnue: {s}");
                print_help();
                std::process::exit(2);
            }
            other => {
                if o.input.is_some() {
                    eprintln!("Un seul input supporté pour l’instant (tu as déjà donné {:?})", o.input);
                    std::process::exit(2);
                }
                o.input = Some(other.to_string());
            }
        }
        i += 1;
    }

    if o.input.is_none() {
        eprintln!("Spécifie un input (fichier ou '-' pour stdin).");
        std::process::exit(2);
    }
    o
}

fn load_program_from_stdin() -> Result<vitte_vm::asm::RawProgram, loader::LoaderError> {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).map_err(loader::LoaderError::Io)?;
    loader::load_raw_program(&buf[..])
}

fn load_program_from_file(path: &str) -> Result<vitte_vm::asm::RawProgram, loader::LoaderError> {
    loader::load_raw_program_from_path(path)
}

fn hex_dump(bytes: &[u8], start_addr: Option<u32>, width: usize, max_bytes: usize) -> String {
    let mut s = String::new();
    let w = width.max(1);
    let limit = bytes.len().min(max_bytes);
    let data = &bytes[..limit];
    let mut offset: usize = 0;
    while offset < data.len() {
        let line = &data[offset..(offset + w).min(data.len())];
        if let Some(addr) = start_addr {
            let addr = (addr as usize) + offset;
            s.push_str(&format!("{addr:08X}  "));
        } else {
            s.push_str(&format!("{:08X}  ", offset));
        }
        for j in 0..w {
            if j < line.len() {
                s.push_str(&format!("{:02X} ", line[j]));
            } else {
                s.push_str("   ");
            }
            if j == 7 { s.push(' '); }
        }
        s.push(' ');
        for &b in line {
            let c = if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' };
            s.push(c);
        }
        s.push('\n');
        offset += w;
    }
    if limit < bytes.len() {
        s.push_str(&format!("... ({} octets non montrés)\n", bytes.len() - limit));
    }
    s
}

fn main() {
    let opts = parse_args();
    let input = opts.input.as_ref().unwrap();

    // Charge le programme
    let prog = if input == "-" {
        match load_program_from_stdin() {
            Ok(p) => p,
            Err(e) => { eprintln!("Erreur de chargement (stdin): {e}"); std::process::exit(1); }
        }
    } else {
        match load_program_from_file(input) {
            Ok(p) => p,
            Err(e) => { eprintln!("Erreur de chargement ({}): {e}", input); std::process::exit(1); }
        }
    };

    // Désassemblement (table par défaut)
    let table = OpcodeTable::new_default();
    let mut out = String::new();

    // En-tête optionnel
    if opts.stats {
        let kints = prog.const_pool.ints.len();
        let kf = prog.const_pool.floats.len();
        let ks = prog.const_pool.strings.len();
        let ndata = prog.data_blobs.len();
        let ncode = prog.code.len();
        out.push_str(&format!(
            "; ======== VITBC DISASM ========\n; entry_pc: {}\n; consts: ints={} floats={} strings={}\n; data_blobs: {}\n; code_ops: {}\n;\n",
            prog.entry_pc.map(|x| x.to_string()).unwrap_or_else(|| "None".into()),
            kints, kf, ks, ndata, ncode
        ));
    }

    // Source ASM désassemblée
    out.push_str(&asm::disassemble(&prog, &table));
    out.push('\n');

    // Hex dump des blobs .data (optionnel)
    if opts.dump_data && !prog.data_blobs.is_empty() {
        out.push_str("\n; ======== DATA (hex dump) ========\n");
        for (i, blob) in prog.data_blobs.iter().enumerate() {
            let name = blob.name.as_deref().unwrap_or("<anon>");
            let addr = blob.addr;
            out.push_str(&format!(
                "; blob #{:02} name={} len={} addr={}\n",
                i, name, blob.bytes.len(),
                addr.map(|a| format!("0x{a:08X}")).unwrap_or_else(|| "None".into())
            ));
            out.push_str(&hex_dump(&blob.bytes, addr, opts.width, opts.max_bytes));
            out.push('\n');
        }
    }

    // Écrit résultat
    if let Some(path) = opts.out {
        let p = Path::new(&path);
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("Impossible de créer le dossier parent {:?}: {e}", parent);
                    std::process::exit(1);
                }
            }
        }
        if let Err(e) = File::create(&p).and_then(|mut f| f.write_all(out.as_bytes())) {
            eprintln!("Écriture échouée ({path}): {e}");
            std::process::exit(1);
        }
    } else {
        print!("{out}");
    }
}
