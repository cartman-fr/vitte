
// Generated help list (skeleton)
const HELP: &str = r#"
Vitte Toolchain
USAGE:
  vitte <command> [options]

CORE:
  new, build, run, test, fmt, doc, bench, profile, fuzz, pkg, pgo

EXTENDED:
  vitr, vittest, vitcov, vitlint, vitfix, vitdep, vitsec, vittop, vitgraph,
  vitpack, vitstrip, vitobj, vitdis, vitpm, vitdoctor, vitu, vitdev,
  vitgen, vitdbg, vitcmp, vitmod, vitup, vitsign, vittrace, vitfmt
"#;
fn main(){ println!("{}", HELP); }
