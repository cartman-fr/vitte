use clap::{CommandFactory, Args as ClapArgs};
use clap_complete::{generate, shells::{Bash, Zsh, Fish, PowerShell, Elvish}};
use std::io;
use std::path::PathBuf;

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// Shell cible: bash|zsh|fish|powershell|elvish
    #[arg(long, value_parser = ["bash","zsh","fish","powershell","elvish"])]
    pub shell: String,
    /// Dossier de sortie (stdout si omis)
    #[arg(long)]
    pub out_dir: Option<PathBuf>,
}

pub fn exec(args: Args) -> color_eyre::Result<()> {
    let mut cmd = crate::Cli::command(); // nécessite pub struct Cli

    let mut writer: Box<dyn io::Write> = if let Some(dir) = args.out_dir {
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("vitte");
        Box::new(std::fs::File::create(path)?)
    } else {
        Box::new(io::stdout())
    };

    match args.shell.as_str() {
        "bash"       => generate(Bash,       &mut cmd, "vitte", &mut writer),
        "zsh"        => generate(Zsh,        &mut cmd, "vitte", &mut writer),
        "fish"       => generate(Fish,       &mut cmd, "vitte", &mut writer),
        "powershell" => generate(PowerShell, &mut cmd, "vitte", &mut writer),
        "elvish"     => generate(Elvish,     &mut cmd, "vitte", &mut writer),
        _ => unreachable!(),
    };
    Ok(())
}