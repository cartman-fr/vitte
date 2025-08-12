use vitte_backend_api::{MirBuilder, Ty, Backend};
use vitte_backend_cranelift::build::CraneliftBackend;

fn main(){
    // Build MIR: print(42); return;
    let mut mb = MirBuilder::new("main", Ty::Unit);
    let c = mb.const_i64(42);
    mb.print(c);
    mb.ret(None);
    let f = mb.finish();

    let mut be = CraneliftBackend::new("x86_64-unknown-linux-gnu");
    let res = be.compile_fn(&f, "target/vitte-backend-mini").expect("compile");
    eprintln!("artifact: {} | log: {}", res.artifact, res.log);
}
