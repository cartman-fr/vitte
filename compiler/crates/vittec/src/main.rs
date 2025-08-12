use std::env;
use std::path::Path;
use vitte_compiler as vc;

fn usage()->!{
    eprintln!("vittec <run|llc|build> <file.vitte>");
    std::process::exit(2);
}

fn main(){
    let mut args = env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| usage());
    match cmd.as_str(){
        "run" => {
            let file = args.next().unwrap_or_else(|| usage());
            vc::run_vm(Path::new(&file));
        }
        "llc" => {
            let file = args.next().unwrap_or_else(|| usage());
            let ir = vc::emit_llvm_ir(Path::new(&file));
            println!("{}", ir);
        }
        "build" => {
            let file = args.next().unwrap_or_else(|| usage());
            let ch = vc::compile_to_bytecode(Path::new(&file));
            let mut buf = Vec::new();
            buf.extend_from_slice(&(ch.code.len() as u32).to_le_bytes());
            buf.extend_from_slice(&ch.code);
            buf.extend_from_slice(&(ch.const_i64.len() as u32).to_le_bytes());
            for n in ch.const_i64 { buf.extend_from_slice(&n.to_le_bytes()); }
            buf.extend_from_slice(&(ch.const_str.len() as u32).to_le_bytes());
            for s in ch.const_str { let b=s.into_bytes(); buf.extend_from_slice(&(b.len() as u32).to_le_bytes()); buf.extend_from_slice(&b); }
            std::fs::write("out.vbc", buf).expect("write");
            eprintln!("Bytecode écrit: out.vbc");
        }
        _ => usage(),
    }
}
