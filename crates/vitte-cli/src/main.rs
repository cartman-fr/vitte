use std::env;
use std::fs;
use vitte_ast::{tokenize, Parser};
use vitte_fmt as fmt;
use vitte_doc as doc;
use vitte_infer as infer;
use vitte_bytecode as bc;
use vitte_vm as vm;
use vitte_codegen_llvm as llc;

fn usage()->!{
    eprintln!("vitte <run|fmt|doc|llc|build|exec|infer|lsp> [--backend=interp|vm|llvm] ...");
    std::process::exit(2);
}

fn main(){
    let mut args = env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| usage());
    match cmd.as_str(){
        "run" => {
            let mut backend = String::from("interp");
            if let Some(flag) = args.clone().next() { if flag.starts_with("--backend="){ backend = flag.split('=').nth(1).unwrap().to_string(); let _=args.next(); } }
            let file = args.next().unwrap_or_else(|| { eprintln!("run <file> [--backend=interp|vm|llvm]"); std::process::exit(2); });
            let src = fs::read_to_string(&file).expect("read");
            run_backend(&backend, &file, &src);
        }
        "fmt" => {
            let file = args.next().unwrap_or_else(|| { eprintln!("fmt <file>"); std::process::exit(2); });
            let src = fs::read_to_string(&file).expect("read");
            let toks = tokenize(&src); let mut p = Parser::new(toks); let prog = p.parse_program().expect("parse");
            println!("{}", fmt::format(&prog));
        }
        "doc" => {
            let input = args.next().unwrap_or_else(|| { eprintln!("doc <in.vitte> <out.html>"); std::process::exit(2); });
            let output = args.next().unwrap_or_else(|| { eprintln!("doc <in.vitte> <out.html>"); std::process::exit(2); });
            let src = fs::read_to_string(&input).expect("read");
            fs::write(&output, doc::generate(&src)).expect("write"); eprintln!("Doc écrite: {}", output);
        }
        "llc" => {
            let file = args.next().unwrap_or_else(|| { eprintln!("llc <file>"); std::process::exit(2); });
            let src = fs::read_to_string(&file).expect("read");
            let toks = tokenize(&src); let mut p = Parser::new(toks); let prog = p.parse_program().expect("parse");
            let ir = llc::emit_ir(&prog, &file);
            println!("{}", ir);
        }
        "build" => {
            let input = args.next().unwrap_or_else(|| { eprintln!("build <in.vitte> <out.vbc>"); std::process::exit(2); });
            let output = args.next().unwrap_or_else(|| { eprintln!("build <in.vitte> <out.vbc>"); std::process::exit(2); });
            let src = fs::read_to_string(&input).expect("read");
            let toks = tokenize(&src); let mut p = Parser::new(toks); let prog = p.parse_program().expect("parse");
            let chunk = bc::compile(&prog);
            let mut buf = Vec::new();
            buf.extend_from_slice(&(chunk.code.len() as u32).to_le_bytes());
            buf.extend_from_slice(&chunk.code);
            buf.extend_from_slice(&(chunk.const_i64.len() as u32).to_le_bytes());
            for n in chunk.const_i64 { buf.extend_from_slice(&n.to_le_bytes()); }
            buf.extend_from_slice(&(chunk.const_str.len() as u32).to_le_bytes());
            for s in chunk.const_str { let b=s.into_bytes(); buf.extend_from_slice(&(b.len() as u32).to_le_bytes()); buf.extend_from_slice(&b); }
            fs::write(&output, buf).expect("write");
            eprintln!("Bytecode: {}", output);
        }
        "exec" => {
            let file = args.next().unwrap_or_else(|| { eprintln!("exec <file.vbc>"); std::process::exit(2); });
            let data = fs::read(&file).expect("read");
            let mut off=0usize;
            let mut rd_u32 = |d:&[u8]|{ let mut b=[0u8;4]; b.copy_from_slice(&d[off..off+4]); off+=4; u32::from_le_bytes(b) };
            let code_len = rd_u32(&data) as usize;
            let code = data[off..off+code_len].to_vec(); off+=code_len;
            let mut chunk = bc::Chunk{ code, const_i64: vec![], const_str: vec![] };
            let i64len = rd_u32(&data) as usize;
            for _ in 0..i64len { let mut b=[0u8;8]; b.copy_from_slice(&data[off..off+8]); off+=8; chunk.const_i64.push(i64::from_le_bytes(b)); }
            let strlen = rd_u32(&data) as usize;
            for _ in 0..strlen { let l = rd_u32(&data) as usize; let s = String::from_utf8(data[off..off+l].to_vec()).unwrap(); off+=l; chunk.const_str.push(s); }
            let mut m = vm::VM::new(); m.run(&chunk);
        }
        "infer" => {
            let file = args.next().unwrap_or_else(|| { eprintln!("infer <file>"); std::process::exit(2); });
            let src = fs::read_to_string(&file).expect("read");
            let toks = tokenize(&src); let mut p = Parser::new(toks); let prog = p.parse_program().expect("parse");
            let mut i = infer::Infer::new(); let mut env = std::collections::HashMap::new();
            let t = infer::infer_expr(&mut i, &mut env, &prog);
            println!("{:?}", t);
        }
        "lsp" => { vitte_lsp::serve_stdio(); }
        _ => usage(),
    }
}

fn run_backend(backend:&str, file:&str, src:&str){
    let toks = vitte_ast::tokenize(src);
    let mut p = Parser::new(toks);
    let prog = match p.parse_program(){ Ok(x)=>x, Err(e)=>{ eprintln!("Parse error: {}", e); return; } };
    match backend {
        "interp" | "vm" => {
            let ch = bc::compile(&prog);
            let mut m = vm::VM::new(); m.run(&ch);
        }
        "llvm" => {
            let ir = llc::emit_ir(&prog, file);
            println!("{}", ir);
        }
        _ => { eprintln!("backend inconnu: {}", backend); }
    }
}
