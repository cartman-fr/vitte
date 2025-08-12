use clap::Parser;
use vitte_backend_api::{Backend, MirBuilder, Ty};
use vitte_backend_cranelift::build::CraneliftBackend;
use vitte_backend_llvm::build::LlvmBackend;

#[derive(Parser, Debug)]
#[command(name="vitte-backend-demo")]
struct Args {
    #[arg(long, default_value="cranelift")] backend: String,
    #[arg(long, default_value="x86_64-unknown-linux-gnu")] triple: String,
    #[arg(long, default_value="target/vitte-demo")] out: String,
    /// Immediate value to print
    #[arg(long, default_value_t=123)] imm: i64,
}

fn main(){
    let args = Args::parse();
    let mut mb = MirBuilder::new("main", Ty::Unit);
    let v = mb.const_i64(args.imm);
    mb.print(v);
    mb.ret(None);
    let f = mb.finish();

    let res = if args.backend=="cranelift" {
        let mut be = CraneliftBackend::new(&args.triple);
        be.compile_fn(&f, &args.out)
    } else {
        let mut be = LlvmBackend::new(&args.triple, "O2");
        be.compile_fn(&f, &args.out)
    }.expect("compile");

    eprintln!("artifact: {} | log: {}", res.artifact, res.log);
}
