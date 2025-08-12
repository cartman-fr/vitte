use clap::Args as ClapArgs;
use std::path::PathBuf;
use color_eyre::eyre::Result;
use vitte_compiler::{Compiler, CompilerConfig, OutputKind};

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// Fichier source .vitte à compiler (exclu si --inline est fourni)
    pub input: Option<PathBuf>,

    /// Source inline à compiler (sinon lire depuis `input`)
    #[arg(long)]
    pub inline: Option<String>,

    /// Dossier de sortie pour l'artefact .vbc (défaut: .)
    #[arg(long)]
    pub out_dir: Option<PathBuf>,

    /// Chemin du binaire `vitte` à utiliser (sinon VITTE_BIN ou 'vitte')
    #[arg(long)]
    pub vitte_bin: Option<PathBuf>,
}

pub fn exec(args: Args) -> Result<()> {
    let mut cfg = CompilerConfig::default();
    if let Some(p) = args.vitte_bin { cfg.vitte_bin = Some(p.into()); }
    let out_dir = args.out_dir.unwrap_or_else(|| std::env::current_dir().unwrap());

    let c = Compiler::new(cfg);
    let prod = if let Some(src) = args.inline {
        c.compile_str(&src, &out_dir, OutputKind::BytecodeVbc)?
    } else {
        let input = args.input.expect("précise un fichier .vitte ou --inline");
        c.compile_file(&input, &out_dir, OutputKind::BytecodeVbc)?
    };
    if let Some(bytes) = prod.output {
        // Si compilation inline, écrire un nom par défaut
        if args.inline.is_some() {
            let dst = out_dir.join("inline.vbc");
            std::fs::write(&dst, &bytes)?;
            println!("{}", dst.display());
        } else if let Some(inp) = args.input {
            let mut dst = inp.file_name().map(|s| s.to_owned()).unwrap();
            // remplace extension par .vbc
            let name = dst.to_string_lossy().to_string();
            let stem = name.trim_end_matches(".vitte");
            let out_name = if stem == name { format!("{}.vbc", name) } else { format!("{}.vbc", stem) };
            let path = out_dir.join(out_name);
            std::fs::write(&path, &bytes)?;
            println!("{}", path.display());
        }
    }
    Ok(())
}