use std::process::Command;
use std::path::{Path, PathBuf};
use color_eyre::eyre::{Result, eyre};
use crate::config::CompilerConfig;
use crate::util::fs;

/// Backend qui appelle le binaire `vitte` pour compiler en bytecode `.vbc`.
#[derive(Debug, Clone)]
pub struct BytecodeCli {
    bin: String,
}

impl BytecodeCli {
    pub fn new(cfg: &CompilerConfig) -> Self {
        let bin = cfg.vitte_bin.as_ref().map(|s| s.as_str()).unwrap_or("vitte").to_string();
        Self { bin }
    }

    /// Compile un fichier source vers un artefact .vbc et retourne son contenu binaire.
    pub fn compile_file(&self, input: &Path, out_dir: &Path) -> Result<Vec<u8>> {
        std::fs::create_dir_all(out_dir)?;
        // Stratégie: on laisse `vitte bc` produire `input.vbc` à côté du source,
        // puis on copie dans out_dir pour garder l'artefact.
        let status = Command::new(&self.bin)
            .arg("bc")
            .arg(input)
            .status()?;
        if !status.success() {
            return Err(eyre!("échec de `{} bc {}` (status {:?})", self.bin, input.display(), status));
        }
        let mut out = PathBuf::from(input);
        out.set_extension("vbc");
        if !out.exists() {
            return Err(eyre!("artefact bytecode introuvable: {}", out.display()));
        }
        let bytes = std::fs::read(&out)?;
        // copie dans out_dir avec le même nom
        let dst = out_dir.join(out.file_name().unwrap());
        fs::write_all(&dst, &bytes)?;
        Ok(bytes)
    }

    /// Compile du code en mémoire (écrit un tmp sur disque).
    pub fn compile_str(&self, source: &str, out_dir: &Path) -> Result<Vec<u8>> {
        let tmp = fs::tmp_file("vitte-src", "vitte");
        fs::write_all(&tmp, source.as_bytes())?;
        let res = self.compile_file(&tmp, out_dir);
        // best-effort cleanup
        let _ = std::fs::remove_file(&tmp);
        res
    }
}