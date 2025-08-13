//! vitte-cli/src/lib.rs — CLI lib pour Vitte
//!
//! Sous-commandes :
//!   - build  : compile un projet .vit (via vitte-compiler si feature "compiler")
//!   - run    : exécute un .vitbc (via vitte-vm si feature "vm")
//!   - disasm : désassemble un .vitbc (via vitte-core si feature "core")
//!   - test   : exécute les tests du projet (découverte basique)
//!
//! Conçu pour compiler même si les crates core/compiler/vm ne sont pas prêtes :
//! les intégrations sont sous features facultatives.

use std::{fs, path::PathBuf};
use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use camino::{Utf8Path, Utf8PathBuf};

/// Point d’entrée du binaire (à appeler depuis src/main.rs)
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Build { manifest, release } => cmd_build(manifest, release),
        Cmd::Run { file } => cmd_run(file),
        Cmd::Disasm { file } => cmd_disasm(file),
        Cmd::Test { manifest, filter } => cmd_test(manifest, filter),
    }
}

#[derive(Parser, Debug)]
#[command(name="vitte", version, about="Vitte language tool")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Compile le projet à partir d’un manifest vitte.toml
    Build {
        /// Chemin vers vitte.toml
        #[arg(default_value = "vitte.toml")]
        manifest: PathBuf,
        /// Profil release (opt-level 3, LTO…)
        #[arg(long)]
        release: bool,
    },
    /// Exécute un fichier bytecode .vitbc
    Run {
        /// Fichier .vitbc (ou .vit si compile&run implémenté)
        file: PathBuf,
    },
    /// Désassemble un bytecode .vitbc
    Disasm {
        /// Fichier .vitbc à désassembler
        file: PathBuf,
    },
    /// Lance les tests du projet (découverte dans tests/)
    Test {
        /// Chemin du manifest
        #[arg(default_value = "vitte.toml")]
        manifest: PathBuf,
        /// Filtre nom de test
        #[arg(long)]
        filter: Option<String>,
    },
}

/// Manifest minimal pour un projet Vitte.
#[derive(Debug, Deserialize)]
struct Manifest {
    package: Package,
    #[serde(default)]
    bin: Option<Bin>,
    #[serde(default)]
    lib: Option<Lib>,
    #[serde(default)]
    targets: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    #[serde(default = "default_version")]
    version: String,
    #[serde(default = "default_edition")]
    edition: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Bin {
    main: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Lib {
    path: String,
    #[serde(default)]
    name: Option<String>,
}

fn default_version() -> String { "0.1.0".into() }
fn default_edition() -> String { "2025".into() }

fn read_manifest(path: &Utf8Path) -> Result<Manifest> {
    let s = fs::read_to_string(path).with_context(|| format!("lecture {}", path))?;
    let m: Manifest = toml::from_str(&s).with_context(|| "TOML invalide")?;
    Ok(m)
}

fn cmd_build(manifest: PathBuf, release: bool) -> Result<()> {
    let manifest = Utf8PathBuf::from_path_buf(manifest).map_err(|_| anyhow!("chemin invalide"))?;
    let m = read_manifest(&manifest)?;
    let root = manifest
        .parent()
        .context("manifest sans parent ?")?
        .to_path_buf();

    let profile = if release { "release" } else { "dev" };
    eprintln!("🏗️  Build `{}` v{}  (profile: {profile})", m.package.name, m.package.version);

    let mut built_any = false;

    // Build lib si présente
    if let Some(lib) = &m.lib {
        let lib_path = root.join(&lib.path);
        ensure_exists(&lib_path, "lib")?;
        // === Codegen vers bytecode (si compiler disponible) ===
        #[cfg(feature = "compiler")]
        {
            built_any = true;
            build_one_source(&lib_path, &root.join("target").join("lib.vitbc"))?;
        }
        #[cfg(not(feature = "compiler"))]
        {
            eprintln!("ℹ️  feature `compiler` absente → lib non compilée (squelette).");
        }
    }

    // Build bin si présent
    if let Some(bin) = &m.bin {
        let main_path = root.join(&bin.main);
        ensure_exists(&main_path, "bin")?;
        #[cfg(feature = "compiler")]
        {
            built_any = true;
            let out_name = bin.name.clone().unwrap_or_else(|| m.package.name.clone());
            let out_path = root.join("target").join(format!("{out_name}.vitbc"));
            build_one_source(&main_path, &out_path)?;
            eprintln!("✅  Binaire bytecode généré → {}", out_path);
        }
        #[cfg(not(feature = "compiler"))]
        eprintln!("ℹ️  feature `compiler` absente → binaire non compilé (squelette).");
    }

    if !built_any {
        eprintln!("⚠️  Rien à construire (pas de `compiler` activé).");
    }

    Ok(())
}

fn cmd_run(file: PathBuf) -> Result<()> {
    let file = Utf8PathBuf::from_path_buf(file).map_err(|_| anyhow!("chemin invalide"))?;
    ensure_exists(&file, "bytecode")?;

    #[cfg(feature = "vm")]
    {
        use std::fs;
        use vitte_core::bytecode::chunk::Chunk;
        let bytes = fs::read(&file)?;
        let chunk = Chunk::from_bytes(&bytes).context("chargement chunk")?;
        let mut vm = vitte_vm::Vm::new();
        vm.run(&chunk).context("exécution VM")?;
        eprintln!("✅  Exécution OK");
        return Ok(());
    }
    #[cfg(not(feature = "vm"))]
    {
        Err(anyhow!("La feature `vm` n’est pas activée (squelette)."))
    }
}

fn cmd_disasm(file: PathBuf) -> Result<()> {
    let file = Utf8PathBuf::from_path_buf(file).map_err(|_| anyhow!("chemin invalide"))?;
    ensure_exists(&file, "bytecode")?;

    #[cfg(feature = "core")]
    {
        use std::fs;
        use vitte_core::bytecode::chunk::Chunk;
        use vitte_core::disasm::disassemble_full;

        let bytes = fs::read(&file)?;
        let chunk = Chunk::from_bytes(&bytes).context("chargement chunk")?;
        let title = file.file_name().unwrap_or("chunk");
        let out = disassemble_full(&chunk, title);
        println!("{out}");
        return Ok(());
    }
    #[cfg(not(feature = "core"))]
    {
        Err(anyhow!("La feature `core` n’est pas activée (squelette)."))
    }
}

fn cmd_test(manifest: PathBuf, filter: Option<String>) -> Result<()> {
    let manifest = Utf8PathBuf::from_path_buf(manifest).map_err(|_| anyhow!("chemin invalide"))?;
    let root = manifest
        .parent()
        .context("manifest sans parent ?")?
        .to_path_buf();
    let tests_dir = root.join("tests");
    if !tests_dir.exists() {
        eprintln!("ℹ️  Pas de dossier `tests/` → rien à faire.");
        return Ok(());
    }

    let mut count = 0usize;
    for entry in walk(&tests_dir)? {
        if entry.extension().map(|e| e == "vit").unwrap_or(false) {
            if let Some(f) = &filter {
                if !entry.to_string_lossy().contains(f) { continue; }
            }
            eprintln!("🧪  Test: {}", entry);
            // MVP : pour l’instant on “valide” symboliquement.
            // Quand le compiler sera branché :
            //   1) compiler .vit -> .vitbc
            //   2) exécuter via VM
            //   3) comparer stdout attendu (e.g., fichier .out)
            count += 1;
        }
    }
    eprintln!("✅  {count} test(s) découverts.");
    Ok(())
}

#[cfg(feature = "compiler")]
fn build_one_source(src: &Utf8Path, out: &Utf8Path) -> Result<()> {
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    use vitte_compiler as compiler;
    use vitte_core::bytecode::chunk::Chunk;

    fs::create_dir_all(out.parent().unwrap())?;
    // Placeholder: tant que le compiler n’est pas codé,
    // on génère un Chunk vide “valide” pour la chaîne outillage.
    let mut chunk = Chunk::new(vitte_core::bytecode::chunk::ChunkFlags { stripped: false });
    let _ = src; // plus tard: compiler::compile_file(src) -> Chunk

    let bytes = chunk.to_bytes();
    let mut f = fs::File::create(out)?;
    f.write_all(&bytes)?;
    Ok(())
}

fn ensure_exists(path: &Utf8Path, what: &str) -> Result<()> {
    if !path.exists() {
        Err(anyhow!("{what} introuvable: {path}"))
    } else {
        Ok(())
    }
}

fn walk(dir: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir)? {
        let e = e?;
        let p = Utf8PathBuf::from_path_buf(e.path()).map_err(|_| anyhow!("UTF-8 path"))?;
        if p.is_dir() {
            out.extend(walk(&p)?);
        } else {
            out.push(p);
        }
    }
    Ok(out)
}
