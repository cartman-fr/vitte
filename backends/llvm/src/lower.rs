//! Lower MIR -> pseudo-LLVM IR (string format).
use vitte_backend_api::{MirFn, MirInst};
pub fn to_pseudo_llvm(f: &MirFn) -> String {
    let mut s = String::new();
    s.push_str(&format!("define @{}() {{\n", f.name));
    for b in &f.blocks {
        s.push_str(&format!("  ; block {}\n", b.id.0));
        for i in &b.insts {
            s.push_str(&format!("  ; {:?}\n", i)); // simple debug print
        }
    }
    s.push_str("}\n");
    s
}
