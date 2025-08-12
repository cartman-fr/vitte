use vitte_backend_api::{MirBuilder, Ty};
use vitte_backend_llvm::lower::to_pseudo_llvm;

#[test]
fn pseudo_ir_mentions_function_name() {
    let mut mb = MirBuilder::new("main", Ty::Unit);
    mb.ret(None);
    let f = mb.finish();
    let ir = to_pseudo_llvm(&f);
    assert!(ir.contains("define @main"));
}
