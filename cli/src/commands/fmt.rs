
use color_eyre::eyre::Result;
use std::path::Path;
use crate::util;
use crate::runtime::{Parser, tokenize};
use crate::pretty;

pub fn format_file(path: &Path) -> Result<()> {
    let src = util::read(path)?;
    let toks = tokenize(&src);
    let mut p = Parser::new(toks);
    let prog = p.parse_program()?;
    let s = pretty::format(&prog, &pretty::Cfg::default());
    println!("{}", s);
    Ok(())
}
