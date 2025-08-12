
use color_eyre::eyre::Result;
use std::time::Instant;
use std::path::Path;
use crate::util;
use crate::runtime::{Parser, tokenize, eval_with_capture};

pub fn bench(file: &Path, iters: u64) -> Result<()> {
    let src = util::read(file)?;
    let toks = tokenize(&src);
    let mut p = Parser::new(toks);
    let prog = p.parse_program()?;
    let t0 = Instant::now();
    for _ in 0..iters { let _ = eval_with_capture(&prog, true, path.parent())?; }
    let dt = t0.elapsed();
    println!("{} runs in {:?}", iters, dt);
    println!("{:.3} µs / run", dt.as_secs_f64()*1e6 / (iters as f64));
    Ok(())
}
