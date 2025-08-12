use vitte_backend_api::{MirBuilder, Ty};
use vitte_backend_cranelift::lower::to_pseudo_clif;

#[test]
fn pseudo_clif_contains_iconst_and_return() {
    let mut mb = MirBuilder::new("main", Ty::Unit);
    let c = mb.const_i64(7);
    mb.print(c);
    mb.ret(None);
    let f = mb.finish();
    let clif = to_pseudo_clif(&f);
    assert!(clif.contains("iconst.i64 7"));
    assert!(clif.contains("return"));
}
