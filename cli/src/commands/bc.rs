
use color_eyre::eyre::Result;
use std::path::{Path, PathBuf};
use crate::bytecode;

pub fn compile(file: &Path, out: Option<&PathBuf>) -> Result<()> {
    let chunk = bytecode::compile_file(file)?;
    let bin = bincode::serialize(&chunk.ops).expect("serialize");
    let out_path = out.cloned().unwrap_or_else(|| {
        let mut p = file.to_path_buf(); p.set_extension("vbc"); p
    });
    std::fs::write(&out_path, bin)?;
    eprintln!("Bytecode écrit: {}", out_path.display());
    Ok(())
}
