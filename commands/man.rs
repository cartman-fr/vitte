
use clap::CommandFactory;
use clap_mangen::Man;
use color_eyre::eyre::Result;
use std::path::Path;

pub fn generate(outdir: &Path) -> Result<()> {
    std::fs::create_dir_all(outdir)?;
    let cmd = crate::Cli::command();
    let man = Man::new(cmd);
    let mut out = Vec::new();
    man.render(&mut out)?;
    let path = outdir.join("vitte.1");
    std::fs::write(&path, out)?;
    eprintln!("Page man écrite: {}", path.display());
    Ok(())
}
