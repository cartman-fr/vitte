use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use vitte_ast::{tokenize, Parser, Expr};
use vitte_fmt as fmt;
use vitte_doc as doc;
use vitte_infer as infer;
use vitte_bytecode as bc;
use vitte_vm as vm;
use vitte_codegen_llvm as llc;

fn usage()->!{
    eprintln!("vitte <run|fmt|doc|llc|build|exec|infer|lsp> [--backend=vm|llvm] ...");
    std::process::exit(2);
}

fn load_inline(file:&Path) -> Expr {
    let src = fs::read_to_string(file).expect("read");
    let toks = tokenize(&src); let mut p = Parser::new(toks);
    let prog = p.parse_program().expect("parse");
    expand_imports(file.parent().unwrap_or(Path::new(".")), prog)
}

fn expand_imports(base:&Path, e:Expr) -> Expr {
    match e {
        Expr::Prog(xs) => {
            let mut out = vec![];
            for x in xs {
                match x {
                    Expr::Import(path) => {
                        let p = base.join(path);
                        let sub = load_inline(&p);
                        if let Expr::Prog(ys) = sub { out.extend(ys); } else { out.push(sub); }
                    }
                    other => out.push(expand_imports(base, other)),
                }
            }
            Expr::Prog(out)
        }
        other => other,
    }
}

fn main(){
    let mut args = env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| usage());
    match cmd.as_str(){
        "run" => {
            let backend = "vm".to_string();
            let file = args.next().unwrap_or_else(|| { eprintln!("run <file>"); std::process::exit(2); });
            let prog = load_inline(Path::new(&file));
            match backend.as_str() {
                "vm" => { let ch = bc::compile(&prog); let mut m = vm::VM::new(); m.run(&ch); }
                "llvm" => { let ir = llc::emit_ir(&prog, &file); println!("{}", ir); }
                _ => {}
            }
        }
        "fmt" => {
            let file = args.next().unwrap_or_else(|| { eprintln!("fmt <file>"); std::process::exit(2); });
            let prog = load_inline(Path::new(&file));
            println!("{}", fmt::format(&prog));
        }
        "doc" => {
            let input = args.next().unwrap_or_else(|| { eprintln!("doc <in.vitte> <out.html>"); std::process::exit(2); });
            let output = args.next().unwrap_or_else(|| { eprintln!("doc <in.vitte> <out.html>"); std::process::exit(2); });
            let prog = load_inline(Path::new(&input));
            let s = fmt::format(&prog);
            fs::write(&output, doc::generate(&s)).expect("write"); eprintln!("Doc écrite: {}", output);
        }
        "llc" => {
            let file = args.next().unwrap_or_else(|| { eprintln!("llc <file>"); std::process::exit(2); });
            let prog = load_inline(Path::new(&file));
            let ir = llc::emit_ir(&prog, &file);
            println!("{}", ir);
        }
        "build" => {
            let input = args.next().unwrap_or_else(|| { eprintln!("build <in.vitte> <out.vbc>"); std::process::exit(2); });
            let output = args.next().unwrap_or_else(|| { eprintln!("build <in.vitte> <out.vbc>"); std::process::exit(2); });
            let prog = load_inline(Path::new(&input));
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
            let prog = load_inline(Path::new(&file));
            let mut i = infer::Infer::default(); let mut env = std::collections::HashMap::new();
            let t = infer::infer_expr(&mut i, &mut env, &prog);
            println!("{:?}", t);
        }
        "lsp" => { vitte_lsp::serve_stdio(); }
        _ => usage(),
    }
}
