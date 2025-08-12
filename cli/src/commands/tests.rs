
use color_eyre::eyre::{Result, eyre};
use std::fs;
use std::path::{Path, PathBuf};
use crate::util;
use crate::runtime::{Parser, tokenize, eval_with_capture};

pub fn run_all(dir: &Path) -> Result<()> {
    let mut ok = 0usize; let mut ko = 0usize;
    for entry in walk(dir)? {
        let src = util::read(&entry)?;
        let expect_lines: Vec<String> = src.lines()
            .filter_map(|l| l.trim_start().strip_prefix("# EXPECT: ").map(|s| s.to_string()))
            .collect();
        if expect_lines.is_empty() { continue; }

        let toks = tokenize(&src);
        let mut p = Parser::new(toks);
        let prog = p.parse_program()?;

        let out = eval_with_capture(&prog, true, entry.parent())?;
        let expected = expect_lines.join("\n");
        if out.trim() == expected.trim() {
            println!("✅ {}", entry.display());
            ok += 1;
        } else {
            eprintln!("❌ {}", entry.display());
            eprintln!("--- attendu ---\n{}\n--- obtenu ----\n{}\n-------------", expected, out);
            ko += 1;
        }
    }
    eprintln!("Tests OK: {}, KO: {}", ok, ko);
    if ko>0 { Err(eyre!("Des tests ont échoué")) } else { Ok(()) }
}

fn walk(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = vec![];
    for e in fs::read_dir(dir)? {
        let e = e?;
        let p = e.path();
        if p.is_dir() { out.extend(walk(&p)?); }
        else if p.extension().and_then(|s| s.to_str()) == Some("vitte") { out.push(p); }
    }
    Ok(out)
}
