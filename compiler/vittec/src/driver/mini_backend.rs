
use std::fs;
use std::path::Path;
use std::process::Command;

/// Tiny "compiler": detect `print("...")` in `fn main(){ ... }` and bootstrap to C.
/// If `cc` is available, compile to native and emit `<out>.vitx`.
pub fn compile_minimal(src_path: &str, out_stem: &str) -> std::io::Result<()> {
    let src = fs::read_to_string(src_path)?;
    let mut message = "Hello, Vitte!".to_string();
    // naive parse: look for print("...")
    if let Some(i) = src.find("print(") {
        if let Some(j) = src[i..].find(')') {
            let inner = &src[i+6..i+j];
            // crude: strip quotes
            let inner = inner.trim().trim_matches('"');
            if !inner.is_empty() { message = inner.to_string(); }
        }
    }
    let c_code = format!(r#"#include <stdio.h>
int main(){ puts("{msg}"); return 0; }
"#, msg=message.replace("\"","\\\""));
    let c_path = format!("{out}.c", out=out_stem);
    fs::write(&c_path, c_code)?;

    // Try to compile with system cc
    let vitx_path = format!("{out}.vitx", out=out_stem);
    let try_cc = Command::new("sh")
        .arg("-lc")
        .arg(format!("cc -O2 {c} -o {bin} && strip {bin} || true", c=c_path, bin=vitx_path))
        .status();

    match try_cc {
        Ok(status) if status.success() => {
            eprintln!("[vittec-mini] built {vitx}", vitx=vitx_path);
        }
        _ => {
            eprintln!("[vittec-mini] cc not available; left C shim at {c}", c=c_path);
        }
    }
    Ok(())
}
