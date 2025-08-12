use std::path::Path;
use vitte_compiler::{Compiler, CompilerConfig, OutputKind};

#[test]
fn compile_inline_to_vbc() {
    if std::env::var("VITTE_BIN").is_err() {
        eprintln!("(skip) set VITTE_BIN to run this test");
        return;
    }
    let cfg = CompilerConfig::default();
    let c = Compiler::new(cfg);
    let out = tempfile::tempdir().unwrap();
    let p = c.compile_str(r#"print("hi")"#, out.path(), OutputKind::BytecodeVbc).unwrap();
    assert!(p.output.is_some() && !p.output.unwrap().is_empty());
}