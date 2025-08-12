//! Minimal CLI to drive the Cranelift backend with MIR builder.
use vitte_backend_api::{Backend, MirBuilder, Ty};
use crate::build::CraneliftBackend;

pub fn run_demo() {
    let mut mb = MirBuilder::new("main", Ty::Unit);
    let forty_two = mb.const_i64(42);
    mb.print(forty_two);
    mb.ret(None);
    let f = mb.finish();

    let mut be = CraneliftBackend::new("x86_64-unknown-linux-gnu");
    let res = be.compile_fn(&f, "target/vitte-cranelift-demo").expect("compile");
    eprintln!("artifact: {} | log: {}", res.artifact, res.log);
}
