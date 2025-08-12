
use color_eyre::eyre::Result;
use std::io::{self, Write};

pub fn repl() -> Result<()> {
    println!("vitte REPL — ctrl+d pour quitter");
    let mut line = String::new();
    loop {
        line.clear();
        print!(">>> "); io::stdout().flush()?;
        if io::stdin().read_line(&mut line)? == 0 { break; }
        if line.trim().is_empty() { continue; }
        println!("{}", line.trim());
    }
    Ok(())
}
