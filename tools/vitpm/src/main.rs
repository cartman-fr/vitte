// vitpm/src/main.rs — Vitte Package Manager (ultra compact & sans deps externes)
//
// Sous-commandes (usage rapide) :
//   vitpm init [--name <n>] [--version <v>]       # initialise vitpm.toml dans le dossier courant
//   vitpm new <name> [--bin|--lib]                # crée un squelette de projet
//   vitpm add <pkg[@ver]> [--path <dir>]          # ajoute une dépendance (version ou path)
//   vitpm remove <pkg>                            # supprime une dépendance
//   vitpm install                                 # résout & vendorise (vitte_modules/)
//   vitpm update                                  # réinstalle les deps (simple)
//   vitpm build [--release] [--zstd]              # compile src/main.(asm|vit) → target/vitpm/<name>.vitbc
//   vitpm run                                     # build + exécution (feature eval requise côté vitte-core)
//   vitpm scripts run <name> [args...]            # exécute un script défini dans [scripts] du manifest
//   vitpm info | tree                             # infos paquet, arbre des deps
//   vitpm lock                                    # (ré)écrit vitpm.lock depuis vitpm.toml
//   vitpm publish [--out dist/]                   # archive projet (src/, vitpm.toml, lock, README, LICENSE)
//
// Convention de projet :
//   - Manifest :  vitpm.toml   (sections [package], [dependencies], [scripts])
//   - Sources  :  src/main.asm ou src/main.vit
//   - Vendor   :  vitte_modules/<name>-<ver>/ (copie du registry local)
//   - Cible    :  target/vitpm/<pkg>.vitbc
//
// Registre local : variable $VITPM_REGISTRY (défaut ~/.vitte/registry)
//   Chaque paquet est attendu sous : <registry>/<name>/<version>/  (contient src/ ou un bundle .asm/.vit)
//
// NOTE : c’est un PM “local-first” : pas de net. On copie depuis un dossier de registry.
//        Le parser TOML est volontairement minimal (clés/valeurs simples).
//        Ajuste selon tes besoins (ex: sémver, contraintes, etc.)

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use vitte_vm::{
    asm::{self, OpcodeTable},
    loader,
};

#[cfg(feature = "eval")]
use vitte_core::runtime::eval::{eval_chunk, EvalOptions};

const MANIFEST: &str = "vitpm.toml";
const LOCKFILE: &str = "vitpm.lock";
const VENDOR_DIR: &str = "vitte_modules";
const TARGET_DIR: &str = "target/vitpm";

fn main() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_help();
        std::process::exit(1);
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "help" | "-h" | "--help" => print_help(),
        "init" => cmd_init(args),
        "new" => cmd_new(args),
        "add" => cmd_add(args),
        "remove" => cmd_remove(args),
        "install" => cmd_install(args),
        "update" => cmd_update(args),
        "build" => cmd_build(args),
        "run" => cmd_run(args),
        "scripts" => cmd_scripts(args),
        "info" => cmd_info(args),
        "tree" => cmd_tree(args),
        "lock" => cmd_lock(args),
        "publish" => cmd_publish(args),
        other => {
            eprintln!("✖ commande inconnue: {other}");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    eprintln!(
r#"vitpm — Vitte Package Manager

USAGE
  vitpm init [--name <n>] [--version <v>]
  vitpm new <name> [--bin|--lib]
  vitpm add <pkg[@ver]> [--path <dir>]
  vitpm remove <pkg>
  vitpm install
  vitpm update
  vitpm build [--release] [--zstd]
  vitpm run
  vitpm scripts run <name> [args...]
  vitpm info | tree
  vitpm lock
  vitpm publish [--out dist/]

ENV
  VITPM_REGISTRY   Chemin du registre local (défaut ~/.vitte/registry)
"#
    );
}

// ------------------------- Manifest & Lock -------------------------

#[derive(Debug, Clone)]
struct Manifest {
    name: String,
    version: String,
    authors: Vec<String>,
    edition: String,
    deps: BTreeMap<String, DepSpec>,
    scripts: BTreeMap<String, String>,
}
#[derive(Debug, Clone)]
enum DepSpec { Version(String), Path(String) }

#[derive(Debug, Clone)]
struct Lock {
    root_name: String,
    root_version: String,
    deps: BTreeMap<String, String>, // name -> pinned version
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            name: "vitte-project".into(),
            version: "0.1.0".into(),
            authors: vec![whoami()],
            edition: "2021".into(),
            deps: BTreeMap::new(),
            scripts: BTreeMap::new(),
        }
    }
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "you".into())
}

// TOML-lite : parser très simple (clé=val, sections [x], strings entre guillemets)
fn read_manifest(path: &Path) -> io::Result<Manifest> {
    if !path.exists() { return Err(io::Error::new(io::ErrorKind::NotFound, "manifest introuvable")); }
    let mut s = String::new();
    fs::File::open(path)?.read_to_string(&mut s)?;
    parse_manifest(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn write_manifest(path: &Path, m: &Manifest) -> io::Result<()> {
    let mut out = String::new();
    out.push_str("[package]\n");
    out.push_str(&format!("name = \"{}\"\n", m.name));
    out.push_str(&format!("version = \"{}\"\n", m.version));
    out.push_str(&format!("authors = [{}]\n", m.authors.iter().map(|a| format!("\"{}\"", a)).collect::<Vec<_>>().join(", ")));
    out.push_str(&format!("edition = \"{}\"\n\n", m.edition));
    if !m.deps.is_empty() {
        out.push_str("[dependencies]\n");
        for (k, v) in &m.deps {
            match v {
                DepSpec::Version(ver) => out.push_str(&format!("{k} = \"{ver}\"\n")),
                DepSpec::Path(p)      => out.push_str(&format!("{k}.path = \"{p}\"\n")),
            }
        }
        out.push('\n');
    }
    if !m.scripts.is_empty() {
        out.push_str("[scripts]\n");
        for (k, v) in &m.scripts {
            out.push_str(&format!("{k} = \"{}\"\n", escape_toml(v)));
        }
        out.push('\n');
    }
    fs::write(path, out)
}

fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn parse_manifest(src: &str) -> Result<Manifest, String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Sec { Root, Package, Deps, Scripts }
    let mut sec = Sec::Root;
    let mut m = Manifest::default();

    for (ln, line) in src.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') { continue; }
        if t.starts_with('[') && t.ends_with(']') {
            sec = match &t[1..t.len()-1] {
                "package" => Sec::Package,
                "dependencies" => Sec::Deps,
                "scripts" => Sec::Scripts,
                other => return Err(format!("section inconnue [{other}] (ligne {})", ln+1)),
            };
            continue;
        }
        match sec {
            Sec::Package => {
                if let Some((k, v)) = t.split_once('=') {
                    let key = k.trim();
                    let val = v.trim();
                    match key {
                        "name" => m.name = unquote(val).to_string(),
                        "version" => m.version = unquote(val).to_string(),
                        "edition" => m.edition = unquote(val).to_string(),
                        "authors" => {
                            let mut v = val.trim();
                            if v.starts_with('[') && v.ends_with(']') { v = &v[1..v.len()-1]; }
                            let list = v.split(',')
                                .map(|s| unquote(s.trim()).to_string())
                                .filter(|s| !s.is_empty())
                                .collect::<Vec<_>>();
                            if !list.is_empty() { m.authors = list; }
                        }
                        _ => {}
                    }
                }
            }
            Sec::Deps => {
                // supporte :
                //   foo = "1.2.3"
                //   bar.path = "../local/bar"
                if let Some((k, v)) = t.split_once('=') {
                    let key = k.trim();
                    let val = v.trim();
                    if key.ends_with(".path") {
                        let name = key.trim_end_matches(".path").to_string();
                        m.deps.insert(name, DepSpec::Path(unquote(val).to_string()));
                    } else {
                        m.deps.insert(key.to_string(), DepSpec::Version(unquote(val).to_string()));
                    }
                }
            }
            Sec::Scripts => {
                if let Some((k, v)) = t.split_once('=') {
                    m.scripts.insert(k.trim().to_string(), unquote(v.trim()).to_string());
                }
            }
            Sec::Root => {}
        }
    }
    Ok(m)
}

fn read_lock(path: &Path) -> io::Result<Lock> {
    if !path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "lockfile introuvable"));
    }
    let mut s = String::new();
    fs::File::open(path)?.read_to_string(&mut s)?;
    parse_lock(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn write_lock(path: &Path, l: &Lock) -> io::Result<()> {
    let mut out = String::new();
    out.push_str("[root]\n");
    out.push_str(&format!("name = \"{}\"\nversion = \"{}\"\n\n", l.root_name, l.root_version));
    if !l.deps.is_empty() {
        out.push_str("[dependencies]\n");
        for (k, v) in &l.deps {
            out.push_str(&format!("{k} = \"{v}\"\n"));
        }
        out.push('\n');
    }
    fs::write(path, out)
}

fn parse_lock(src: &str) -> Result<Lock, String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Sec { Root, Deps, None }
    let mut sec = Sec::None;
    let mut name = String::new();
    let mut ver = String::new();
    let mut deps = BTreeMap::new();

    for (ln, line) in src.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') { continue; }
        if t.starts_with('[') && t.ends_with(']') {
            sec = match &t[1..t.len()-1] {
                "root" => Sec::Root,
                "dependencies" => Sec::Deps,
                _ => Sec::None,
            };
            continue;
        }
        match sec {
            Sec::Root => {
                if let Some((k, v)) = t.split_once('=') {
                    match k.trim() {
                        "name" => name = unquote(v.trim()).to_string(),
                        "version" => ver = unquote(v.trim()).to_string(),
                        _ => {}
                    }
                }
            }
            Sec::Deps => {
                if let Some((k, v)) = t.split_once('=') {
                    deps.insert(k.trim().to_string(), unquote(v.trim()).to_string());
                }
            }
            Sec::None => {}
        }
    }
    if name.is_empty() || ver.is_empty() {
        return Err("lockfile incomplet [root]".into());
    }
    Ok(Lock { root_name: name, root_version: ver, deps })
}

fn unquote(s: &str) -> &str {
    let t = s.trim();
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        &t[1..t.len()-1]
    } else { t }
}

// ------------------------- Commands -------------------------

fn cmd_init(mut args: Vec<String>) {
    let name = pop_opt(&mut args, "--name", "-n");
    let ver  = pop_opt(&mut args, "--version", "-V");
    if !args.is_empty() {
        eprintln!("arguments inconnus: {:?}", args);
        std::process::exit(2);
    }
    let mut m = Manifest::default();
    if let Some(n) = name { m.name = n; }
    if let Some(v) = ver { m.version = v; }
    if Path::new(MANIFEST).exists() {
        eprintln!("⚠ {MANIFEST} existe déjà, on ne touche pas.");
    } else {
        write_manifest(Path::new(MANIFEST), &m).expect("écriture manifest");
        println!("✅ créé {MANIFEST}");
    }
    // src minimal
    if !Path::new("src").exists() { fs::create_dir_all("src").unwrap(); }
    let main_candidates = ["src/main.asm", "src/main.vit"];
    if !main_candidates.iter().any(|p| Path::new(p).exists()) {
        fs::write("src/main.asm", "main:\n    NOP\n    RET\n").unwrap();
        println!("📝 écrit src/main.asm");
    }
    // lock
    let lock = Lock { root_name: m.name.clone(), root_version: m.version.clone(), deps: BTreeMap::new() };
    write_lock(Path::new(LOCKFILE), &lock).expect("écriture lock");
    println!("✅ créé {LOCKFILE}");
}

fn cmd_new(mut args: Vec<String>) {
    if args.is_empty() { die("usage: vitpm new <name> [--bin|--lib]"); }
    let name = args.remove(0);
    let is_lib = pop_flag(&mut args, "--lib");
    let _is_bin = pop_flag(&mut args, "--bin");
    if !args.is_empty() { die(&format!("arguments inconnus: {:?}", args)); }

    let root = PathBuf::from(&name);
    if root.exists() { die("le dossier existe déjà"); }
    fs::create_dir_all(root.join("src")).unwrap();

    let mut m = Manifest::default();
    m.name = name.clone();
    write_manifest(&root.join(MANIFEST), &m).unwrap();

    if is_lib {
        fs::write(root.join("src/lib.asm"), "lib_start:\n    NOP\n    RET\n").unwrap();
    } else {
        fs::write(root.join("src/main.asm"), "main:\n    NOP\n    RET\n").unwrap();
    }

    let lock = Lock { root_name: m.name.clone(), root_version: m.version.clone(), deps: BTreeMap::new() };
    write_lock(&root.join(LOCKFILE), &lock).unwrap();

    fs::write(root.join("README.md"), format!("# {}\n\nProjet Vitte.\n", name)).ok();
    println!("✅ projet créé: {name}/");
}

fn cmd_add(mut args: Vec<String>) {
    if args.is_empty() { die("usage: vitpm add <pkg[@ver]> [--path <dir>]"); }
    let spec = args.remove(0);
    let path_opt = pop_opt(&mut args, "--path", "-p");
    if !args.is_empty() { die(&format!("arguments inconnus: {:?}", args)); }

    let mut m = read_manifest(Path::new(MANIFEST)).unwrap_or_else(|_| {
        println!("⚠ pas de manifest → vitpm init");
        Manifest::default()
    });

    if let Some(p) = path_opt {
        m.deps.insert(spec, DepSpec::Path(p));
    } else {
        let (name, ver) = split_name_ver(&spec);
        m.deps.insert(name.into(), DepSpec::Version(ver.into()));
    }
    write_manifest(Path::new(MANIFEST), &m).unwrap();
    println!("✅ dépendance ajoutée. Lance `vitpm install`.");
}

fn cmd_remove(mut args: Vec<String>) {
    if args.is_empty() { die("usage: vitpm remove <pkg>"); }
    let name = args.remove(0);
    if !args.is_empty() { die(&format!("arguments inconnus: {:?}", args)); }

    let mut m = read_manifest(Path::new(MANIFEST)).expect("lit manifest");
    if m.deps.remove(&name).is_some() {
        write_manifest(Path::new(MANIFEST), &m).unwrap();
        println!("✅ supprimé {name} du manifest");
    } else {
        println!("ℹ {name} n’est pas dans les deps");
    }
}

fn cmd_install(args: Vec<String>) {
    if !args.is_empty() { die(&format!("arguments inconnus: {:?}", args)); }
    let m = read_manifest(Path::new(MANIFEST)).expect("manifest");
    fs::create_dir_all(VENDOR_DIR).ok();

    let mut pinned = BTreeMap::new();

    for (name, spec) in &m.deps {
        match spec {
            DepSpec::Path(p) => {
                let src = PathBuf::from(p);
                if !src.exists() { die(&format!("dep path introuvable: {p}")); }
                let dst = PathBuf::from(VENDOR_DIR).join(name);
                if dst.exists() { fs::remove_dir_all(&dst).ok(); }
                copy_dir(&src, &dst).expect("copie dep path");
                pinned.insert(name.clone(), "path".into());
                println!("📦 vendored {name} (path: {p})");
            }
            DepSpec::Version(ver) => {
                let reg = registry_root();
                let pkg_dir = PathBuf::from(&reg).join(name).join(ver);
                if !pkg_dir.exists() { die(&format!("introuvable dans le registre: {pkg_dir:?}")); }
                let dst = PathBuf::from(VENDOR_DIR).join(format!("{name}-{ver}"));
                if dst.exists() { fs::remove_dir_all(&dst).ok(); }
                copy_dir(&pkg_dir, &dst).expect("copie dep registry");
                pinned.insert(name.clone(), ver.clone());
                println!("📦 vendored {name}@{ver}");
            }
        }
    }

    // lock
    let lock = Lock { root_name: m.name.clone(), root_version: m.version.clone(), deps: pinned };
    write_lock(Path::new(LOCKFILE), &lock).unwrap();
    println!("✅ lock mis à jour → {LOCKFILE}");
}

fn cmd_update(args: Vec<String>) {
    if !args.is_empty() { die(&format!("arguments inconnus: {:?}", args)); }
    println!("ℹ update = réinstallation simple depuis le registre local (pas de résolution sémver avancée).");
    cmd_install(vec![]);
}

fn cmd_build(mut args: Vec<String>) {
    let release = pop_flag(&mut args, "--release");
    let zstd = pop_flag(&mut args, "--zstd");
    if !args.is_empty() { die(&format!("arguments inconnus: {:?}", args)); }

    let m = read_manifest(Path::new(MANIFEST)).expect("manifest");
    let src_asm = Path::new("src/main.asm");
    let src_vit = Path::new("src/main.vit");
    let src;
    let is_vit;

    if src_asm.exists() { src = fs::read_to_string(src_asm).unwrap(); is_vit = false; }
    else if src_vit.exists() { src = fs::read_to_string(src_vit).unwrap(); is_vit = true; }
    else { die("aucun input trouvé (src/main.asm ou src/main.vit)"); }

    let asm_src = if is_vit { lower_vit_to_asm(&src) } else { src };

    // assemble
    let assembled = match asm::assemble(&asm_src) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("✖ assemble: {e}\n--- ASM émis ---\n{asm_src}");
            std::process::exit(1);
        }
    };

    fs::create_dir_all(TARGET_DIR).unwrap();
    let out_path = PathBuf::from(TARGET_DIR).join(format!("{}.vitbc", m.name));
    loader::save_raw_program_to_path(&out_path, &assembled.program, zstd).unwrap();
    println!("✅ build → {}", out_path.display());
    if release { println!("ℹ (release flag: pas de diff binaire ici, mais tu peux l’exploiter pour activer --zstd)"); }
}

fn cmd_run(args: Vec<String>) {
    if !args.is_empty() { die(&format!("arguments inconnus: {:?}", args)); }
    #[cfg(not(feature = "eval"))]
    {
        eprintln!("✖ 'vitpm run' requiert vitte-core avec feature 'eval' pour exécuter un Chunk.");
        std::process::exit(2);
    }
    #[cfg(feature = "eval")]
    {
        // On (re)build en mémoire puis on évalue le chunk direct (pas le VITBC).
        let src_asm = Path::new("src/main.asm");
        let src_vit = Path::new("src/main.vit");
        let src;
        let is_vit;
        if src_asm.exists() { src = fs::read_to_string(src_asm).unwrap(); is_vit = false; }
        else if src_vit.exists() { src = fs::read_to_string(src_vit).unwrap(); is_vit = true; }
        else { die("aucun input trouvé (src/main.asm ou src/main.vit)"); }
        let asm_src = if is_vit { lower_vit_to_asm(&src) } else { src };
        let assembled = asm::assemble(&asm_src).unwrap();
        // Convertit RawProgram → Chunk ? Ici on fait simple : on réassemble le ASM dans vitte-core directement
        // (si tu as un pont RawProgram -> Chunk, remplace)
        // Pour l’instant : on simule via un petit chunk "print" si rien.
        let dis = asm::disassemble(&assembled.program, &OpcodeTable::new_default());
        if dis.trim().is_empty() {
            eprintln!("(programme vide)");
            return;
        }
        // Option conservative: on passe par vitte-core::bytecode::chunk::Chunk si tu as un convertisseur.
        // À défaut, on notifie et s’arrête après build ; ou on évalue via un wrapper séparé.
        // Ici : on appelle un évaluateur fictif si tu fournis un Chunk — placeholder minimal :
        eprintln!("ℹ run: un pont RawProgram → Chunk serait nécessaire pour exécuter réellement.\nDésassemblage:\n{dis}");
    }
}

fn cmd_scripts(mut args: Vec<String>) {
    if args.is_empty() || args[0].as_str() != "run" {
        die("usage: vitpm scripts run <name> [args...]");
    }
    args.remove(0);
    if args.is_empty() { die("usage: vitpm scripts run <name> [args...]"); }
    let script_name = args.remove(0);

    let m = read_manifest(Path::new(MANIFEST)).expect("manifest");
    let Some(cmdline) = m.scripts.get(&script_name) else {
        die(&format!("script introuvable: {script_name}"));
    };

    // On exécute via le shell systeme
    let status = if cfg!(target_os = "windows") {
        Command::new("cmd").arg("/C").arg(cmdline).args(args).status()
    } else {
        Command::new("sh").arg("-c").arg(format!("{cmdline} {}", shell_join(args))).status()
    }.expect("échec exécution script");

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}

fn cmd_info(_args: Vec<String>) {
    let m = read_manifest(Path::new(MANIFEST)).expect("manifest");
    println!("package: {} v{}", m.name, m.version);
    println!("authors: {}", m.authors.join(", "));
    if m.deps.is_empty() { println!("dependencies: <none>"); }
    else {
        println!("dependencies:");
        for (k, v) in m.deps {
            match v {
                DepSpec::Version(ver) => println!("  - {k} = \"{ver}\""),
                DepSpec::Path(p)      => println!("  - {k}.path = \"{p}\""),
            }
        }
    }
}

fn cmd_tree(_args: Vec<String>) {
    let m = read_manifest(Path::new(MANIFEST)).expect("manifest");
    let mut seen = BTreeSet::new();
    println!("{} v{}", m.name, m.version);
    for (k, v) in m.deps {
        print!("├─ {} ", k);
        match v {
            DepSpec::Version(ver) => println!("v{}", ver),
            DepSpec::Path(p)      => println!("(path: {})", p),
        }
        if seen.insert(k.clone()) {
            // On pourrait charger les manifests des deps vendorisées pour approfondir.
        }
    }
}

fn cmd_lock(_args: Vec<String>) {
    let m = read_manifest(Path::new(MANIFEST)).expect("manifest");
    let mut deps = BTreeMap::new();
    for (k, v) in &m.deps {
        match v {
            DepSpec::Version(ver) => { deps.insert(k.clone(), ver.clone()); }
            DepSpec::Path(_) => { deps.insert(k.clone(), "path".into()); }
        }
    }
    let l = Lock { root_name: m.name.clone(), root_version: m.version.clone(), deps };
    write_lock(Path::new(LOCKFILE), &l).unwrap();
    println!("✅ lock écrit.");
}

fn cmd_publish(mut args: Vec<String>) {
    let out_dir = pop_opt(&mut args, "--out", "-o").unwrap_or_else(|| "dist".into());
    if !args.is_empty() { die(&format!("arguments inconnus: {:?}", args)); }

    let m = read_manifest(Path::new(MANIFEST)).expect("manifest");
    fs::create_dir_all(&out_dir).ok();

    // Prépare staging
    let stage = PathBuf::from(format!("target/vitpm-pkg/{}-{}", m.name, m.version));
    if stage.exists() { fs::remove_dir_all(&stage).ok(); }
    fs::create_dir_all(&stage).unwrap();

    // Copie fichiers
    copy_if_exists("README.md", &stage.join("README.md"));
    copy_if_exists("LICENSE", &stage.join("LICENSE"));
    copy_if_exists("LICENSE.md", &stage.join("LICENSE.md"));
    fs::create_dir_all(stage.join("src")).ok();
    copy_if_exists("src/main.asm", &stage.join("src/main.asm"));
    copy_if_exists("src/main.vit", &stage.join("src/main.vit"));
    fs::write(stage.join(MANIFEST), fs::read_to_string(MANIFEST).unwrap()).ok();
    if Path::new(LOCKFILE).exists() {
        fs::write(stage.join(LOCKFILE), fs::read_to_string(LOCKFILE).unwrap()).ok();
    }

    // Archive
    fs::create_dir_all(&out_dir).ok();
    let tgz = PathBuf::from(&out_dir).join(format!("{}-{}.tar.gz", m.name, m.version));
    let zip = PathBuf::from(&out_dir).join(format!("{}-{}.zip", m.name, m.version));
    if has_cmd("tar") {
        let status = Command::new("tar")
            .current_dir(stage.parent().unwrap())
            .args(["-czf", tgz.to_str().unwrap(), stage.file_name().unwrap().to_str().unwrap()])
            .status().expect("tar");
        if status.success() { println!("✅ archive: {}", tgz.display()); }
    } else if has_cmd("zip") {
        let status = Command::new("zip")
            .current_dir(stage.parent().unwrap())
            .args(["-rq", zip.to_str().unwrap(), stage.file_name().unwrap().to_str().unwrap()])
            .status().expect("zip");
        if status.success() { println!("✅ archive: {}", zip.display()); }
    } else {
        println!("⚠ ni `tar` ni `zip` trouvés — archive non créée.");
    }
}

// ------------------------- Helpers FS & CLI -------------------------

fn pop_flag(args: &mut Vec<String>, flag: &str) -> bool {
    if let Some(i) = args.iter().position(|a| a == flag) { args.remove(i); true } else { false }
}
fn pop_opt(args: &mut Vec<String>, k1: &str, k2: &str) -> Option<String> {
    if let Some(i) = args.iter().position(|a| a == k1 || a == k2) {
        args.remove(i);
        if i < args.len() { Some(args.remove(i)) } else { None }
    } else { None }
}

fn split_name_ver(spec: &str) -> (&str, &str) {
    if let Some((n, v)) = spec.split_once('@') { (n, v) } else { (spec, "latest") }
}

fn copy_if_exists<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) {
    if Path::new(src.as_ref()).exists() {
        fs::copy(src.as_ref(), dst.as_ref()).ok();
    }
}

fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let e = entry?;
        let ty = e.file_type()?;
        let from = e.path();
        let to = dst.join(e.file_name());
        if ty.is_dir() {
            copy_dir(&from, &to)?;
        } else if ty.is_file() {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn registry_root() -> PathBuf {
    if let Ok(p) = std::env::var("VITPM_REGISTRY") {
        PathBuf::from(p)
    } else {
        let mut home = dirs_home();
        home.push(".vitte/registry");
        home
    }
}

fn dirs_home() -> PathBuf {
    if let Ok(h) = std::env::var("HOME") { PathBuf::from(h) }
    else if let Ok(u) = std::env::var("USERPROFILE") { PathBuf::from(u) }
    else { PathBuf::from(".") }
}

fn has_cmd(name: &str) -> bool {
    if cfg!(target_os = "windows") {
        Command::new("where").arg(name).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
    } else {
        Command::new("which").arg(name).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
    }
}

fn shell_join(args: Vec<String>) -> String {
    let mut out = String::new();
    for (i, a) in args.iter().enumerate() {
        if i > 0 { out.push(' '); }
        if a.chars().any(|c| c.is_whitespace() || "\"'`$".contains(c)) {
            out.push('"');
            out.push_str(&a.replace('"', "\\\""));
            out.push('"');
        } else { out.push_str(a); }
    }
    out
}

// ------------------------- Lower .vit → .asm (MVP) -------------------------

fn lower_vit_to_asm(src: &str) -> String {
    let mut out = String::new();
    let mut strings: Vec<String> = Vec::new();
    let mut entry = "main".to_string();
    let mut has_entry_decl = false;

    for (ln, line) in src.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') { continue; }

        if let Some(rest) = t.strip_prefix("entry ") {
            let name = strip_semicolon(rest).trim();
            if !name.is_empty() { entry = name.to_string(); has_entry_decl = true; }
            continue;
        }
        if let Some(rest) = t.strip_prefix("const ") {
            if let Some((name, rhs)) = rest.split_once('=') {
                out.push_str(&format!(".const {} = {}\n", name.trim(), strip_semicolon(rhs).trim()));
                continue;
            }
        }
        if let Some(rest) = t.strip_prefix("string ") {
            if let Some((name, rhs)) = rest.split_once('=') {
                out.push_str(&format!(".string {} = \"{}\"\n", name.trim(), esc(unquote(rhs.trim()))));
                continue;
            }
        }
        if let Some(rest) = t.strip_prefix("data ") {
            if let Some((name, rhs)) = rest.split_once('=') {
                out.push_str(&format!(".data {} = {}\n", name.trim(), strip_semicolon(rhs).trim()));
                continue;
            }
        }
        if let Some(rest) = t.strip_prefix("org ") {
            out.push_str(&format!(".org {}\n", strip_semicolon(rest).trim()));
            continue;
        }
        if let Some(lbl) = t.strip_prefix("label ") {
            out.push_str(&format!("{}:\n", strip_colon_or_semicolon(lbl).trim()));
            continue;
        }
        if t.ends_with(':') {
            out.push_str(t); out.push('\n'); continue;
        }
        if let Some(rest) = t.strip_prefix("print ") {
            let lit = strip_semicolon(rest).trim();
            strings.push(unquote(lit).to_string());
            let idx = strings.len()-1;
            out.push_str(&format!("    ; print s{idx}\n"));
            out.push_str(&format!("    ; LOADK r0, const:s{idx}\n"));
            out.push_str("    ; PRINT r0\n");
            continue;
        }

        // pass-through minimal
        if let Some(inst) = pass_inst(t) {
            out.push_str("    "); out.push_str(inst); out.push('\n');
        } else {
            out.push_str(&format!("    ; (ignored @{}): {}\n", ln+1, t));
        }
    }

    if !has_entry_decl { out.push_str(".entry main\n"); } else { out.push_str(&format!(".entry {}\n", entry)); }
    for (i, s) in strings.iter().enumerate() {
        out.push_str(&format!(".string s{} = \"{}\"\n", i, esc(s)));
    }
    if !out.lines().any(|l| l.trim_start() == format!("{entry}:")) {
        out.push('\n'); out.push_str(&format!("{entry}:\n"));
    }
    if !out.lines().rev().any(|l| l.trim().eq_ignore_ascii_case("RET")) {
        out.push_str("    RET\n");
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t")
}
fn strip_semicolon(s: &str) -> &str {
    let t = s.trim_end(); if t.ends_with(';') { &t[..t.len()-1] } else { t }
}
fn strip_colon_or_semicolon(s: &str) -> &str {
    let t = s.trim_end(); if t.ends_with(':') || t.ends_with(';') { &t[..t.len()-1] } else { t }
}
fn pass_inst(t: &str) -> Option<&str> {
    const ALLOWED: [&str; 12] = ["NOP","RET","LOADI","LOADK","ADD","SUB","MUL","DIV","JZ","JNZ","JMP","CALL"];
    let up = t.trim_end_matches(';').trim();
    let mnem = up.split_whitespace().next().unwrap_or("");
    if ALLOWED.iter().any(|&k| k.eq_ignore_ascii_case(mnem)) { Some(up) } else { None }
}
