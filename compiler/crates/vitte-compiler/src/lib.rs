use std::fs;
use std::path::{Path, PathBuf};
use vitte_ast::{tokenize, Parser, Expr};
use vitte_hir as hir;
use vitte_resolver as resolver;
use vitte_infer as infer;
use vitte_mir as mir;
use vitte_backend_bytecode as bc;
use vitte_backend_vm as vm;
use vitte_backend_llvm as ll;

pub fn parse_inline(file:&Path)->Expr{
    let src = fs::read_to_string(file).expect("read");
    let toks = tokenize(&src); let mut p = Parser::new(toks);
    let prog = p.parse_program().expect("parse");
    expand_imports(file.parent().unwrap_or(Path::new(".")), prog)
}
fn expand_imports(base:&Path, e:Expr)->Expr{
    match e{
        Expr::Prog(xs)=>{
            let mut out=vec![];
            for x in xs{
                match x{
                    Expr::Import(path)=>{
                        let sub = parse_inline(&base.join(path));
                        if let Expr::Prog(ys)=sub { out.extend(ys); } else { out.push(sub); }
                    }
                    other=> out.push(expand_imports(base, other)),
                }
            }
            Expr::Prog(out)
        }
        other=>other,
    }
}

pub enum Backend{ VM, LLVM }

pub fn compile_to_bytecode(file:&Path)->bc::Chunk{
    let ast = parse_inline(file);
    let h = hir::lower(&ast);
    let _scope = resolver::resolve(&h);
    let mut _inf = infer::Infer::default(); let mut _env = std::collections::HashMap::new();
    let _ = infer::infer_prog(&mut _inf, &mut _env, &h);
    let m = mir::lower(&h);
    bc::compile(&m)
}

pub fn run_vm(file:&Path){
    let ch = compile_to_bytecode(file);
    let mut m = vm::VM::new();
    m.run(&ch);
}

pub fn emit_llvm_ir(file:&Path)->String{
    let ast = parse_inline(file);
    let h = hir::lower(&ast);
    ll::emit_ir(&h, &file.display().to_string())
}
