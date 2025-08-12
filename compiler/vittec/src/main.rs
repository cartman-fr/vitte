
mod driver {
    pub mod mini_backend;
}
use std::env;
fn main(){
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: vittec-min <src.vitte> <out_stem>");
        std::process::exit(2);
    }
    if let Err(e) = driver::mini_backend::compile_minimal(&args[1], &args[2]) {
        eprintln!("[error] {}", e);
        std::process::exit(1);
    }
}
