
use color_eyre::eyre::Result;
use std::path::Path;
use crate::util;
use crate::runtime::{Parser, tokenize, eval, eval_with_capture};

pub fn run_file(path: &Path) -> Result<()> {
    let raw = util::read(path)?;
    let src = util::resolve_imports(&raw, path.parent().unwrap_or(std::path::Path::new(".")))?;
    let toks = tokenize(&src);
    let mut p = Parser::new(toks);
    let prog = p.parse_program()?;
    let _ = eval_with_capture(&prog, false, path.parent())?;
    Ok(())
}
