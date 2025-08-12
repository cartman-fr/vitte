//! Minimal Cranelift backend for Vitte
//! Compiles a fixed AST into a native executable .vitx (Hello World)

pub fn compile_hello(output: &str) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Write;
    // Placeholder: in a real implementation, MIR -> CLIF -> machine code
    let mut f = File::create(output)?;
    writeln!(f, "This is a fake binary for: {}", output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hello() {
        compile_hello("/tmp/hello.vitx").unwrap();
    }
}
