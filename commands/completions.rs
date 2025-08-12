
use clap::CommandFactory;
use clap_complete::{generate, shells};
use std::io;
use color_eyre::eyre::Result;

pub fn generate(shell: crate::Shell) -> Result<()> {
    use crate::Cli;
    let mut cmd = Cli::command();
    match shell {
        crate::Shell::Bash => generate(shells::Bash, &mut cmd, "vitte", &mut io::stdout()),
        crate::Shell::Zsh => generate(shells::Zsh, &mut cmd, "vitte", &mut io::stdout()),
        crate::Shell::Fish => generate(shells::Fish, &mut cmd, "vitte", &mut io::stdout()),
        crate::Shell::PowerShell => generate(shells::PowerShell, &mut cmd, "vitte", &mut io::stdout()),
        crate::Shell::Elvish => generate(shells::Elvish, &mut cmd, "vitte", &mut io::stdout()),
    };
    Ok(())
}
