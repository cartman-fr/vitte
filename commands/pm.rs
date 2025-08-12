
use color_eyre::eyre::{Result, eyre};
use std::path::Path;
use crate::util;

pub fn init(name: &str) -> Result<()> {
    let dir = std::path::Path::new(name);
    std::fs::create_dir_all(dir.join("src"))?;
    util::write(&dir.join("Vitte.toml"), "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\n")?;
    util::write(&dir.join("src/main.vitte"), "print(\"ok\")\n")?;
    eprintln!("Projet créé: {}", dir.display());
    Ok(())
}

pub fn check(manifest: &Path) -> Result<()> {
    let s = util::read(manifest)?;
    if s.contains("[package]") { eprintln!("Manifest OK: {}", manifest.display()); Ok(()) }
    else { Err(eyre!("Manifest invalide: {}", manifest.display())) }
}
