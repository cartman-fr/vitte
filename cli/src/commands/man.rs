use clap::{CommandFactory, Args as ClapArgs};
use clap_mangen::Man;

#[derive(ClapArgs, Debug, Default)]
pub struct Args { }

pub fn exec(_args: Args) -> color_eyre::Result<()> {
    let cmd = crate::Cli::command();
    let man = Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf)?;
    print!("{}", String::from_utf8_lossy(&buf));
    Ok(())
}