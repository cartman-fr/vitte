
use std::path::Path;
use color_eyre::eyre::Result;
use crate::util;
use crate::runtime::{Parser, tokenize, Expr};

#[derive(Debug, Clone)]
pub enum Op {
    Slice,
    Range,
    CreateClosure(String),
    CallValue(usize),
    PushConst(usize),
    CallStatic(String, usize),
    Index,
    SetIndex,
    CallMethod(String, usize),
    CloneRecord(),
    LoadGlobal(String),
    StoreGlobal(String),
    MakeRecord(usize),
    GetField(String),
    SetField(String),
    /// if top-of-stack falsy, jump to absolute ip
    JumpIfFalse(usize),
    /// unconditional jump
    Jump(usize),
    /// drop top-of-stack
    Pop,
    /// duplicate top-of-stack
    Dup,
    /// Pop N values, create a list, push handle
    MakeList(usize),
    PushStr(String),
    PushNum(f64),
    Add, Sub, Mul, Div,
    Print,
}
#[derive(Debug, Clone)]
#[derive(Clone)]
pub struct Function { pub name: String, pub params: Vec<String>, pub ops: Vec<Op> }

#[derive(Clone)]
pub enum Const { Num(f64), Str(String) }

pub struct Chunk {
    pub consts: Vec<Const>,
    pub funcs: Vec<Function>,
    pub funcs: Vec<Function>,
    pub funcs: Vec<Function>, pub ops: Vec<Op> }


fn compile_function(name: String, params: Vec<String>, body: &Expr) -> Function {
    let mut ch = Chunk::default();
    // naive: compile body expression; result stays on stack
    compile_expr(body, &mut ch);
    Function{ name, params, ops: ch.ops }
}

fn intern_const_num(out:&mut Chunk, n:f64)->usize{
    out.consts.push(Const::Num(n));
    out.consts.len()-1
}
fn intern_const_str(out:&mut Chunk, s:&str)->usize{
    out.consts.push(Const::Str(s.to_string()));
    out.consts.len()-1
}
fn free_vars(e: &Expr, params: &[String], acc: &mut std::collections::BTreeSet<String>) {
    use crate::runtime::Expr as E;
    match e {
        Expr::Lam{ params, body } => {
            compile_lambda(out, params, body);
        }
        Expr::Assign{name, e} => {
            if let Expr::Lam{params, body} = &**e {
                compile_lambda(out, params, body);
                out.ops.push(Op::StoreGlobal(name.clone()));
            } else {
                compile_expr(e, out);
                out.ops.push(Op::StoreGlobal(name.clone()));
            }
        }
        Expr::Call{ callee, args } => { /* v8_calls */
            // print special-case
            if let Expr::Var(p) = &**callee {
                if p == "print" {
                    if let Some(arg0) = args.get(0) { compile_expr(arg0, out); }
                    out.ops.push(Op::Print);
                
                else if p == "slice" {
                    if args.len()>=3 { compile_expr(&args[0], out); compile_expr(&args[1], out); compile_expr(&args[2], out); out.ops.push(Op::Slice); }
                }
                else if p == "range" {
                    if args.len()>=2 { compile_expr(&args[0], out); compile_expr(&args[1], out); out.ops.push(Op::Range); }
                }
            } else {
                    // generic: call value
                    compile_expr(callee, out);
                    for a in args { compile_expr(a, out); }
                    out.ops.push(Op::CallValue(args.len()));
                }
            } else if let Expr::Field{ target, name } = &**callee {
                // keep previous method/static lowering (already inserted in v6)
                if let Expr::Var(cls) = &**target {
                    if cls.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) {
                        for a in args { compile_expr(a, out); }
                        out.ops.push(Op::CallStatic(format!("{}::{}", cls, name), args.len()));
                    } else {
                        compile_expr(target, out);
                        for a in args { compile_expr(a, out); }
                        out.ops.push(Op::CallMethod(name.clone(), args.len()));
                    }
                } else {
                    compile_expr(target, out);
                    for a in args { compile_expr(a, out); }
                    out.ops.push(Op::CallMethod(name.clone(), args.len()));
                }
            } else {
                compile_expr(callee, out);
                for a in args { compile_expr(a, out); }
                out.ops.push(Op::CallValue(args.len()));
            }
        }
        E::Var(x) => { if !params.contains(x) && x != "print" { acc.insert(x.clone()); } }
        E::Lam{ params: ps, body } => {
            // nested: free vars of inner excluding its params (we don't add: captured at its own creation)
            free_vars(body, ps, acc);
        }
        E::List(xs) | E::Tuple(xs) => for x in xs { free_vars(x, params, acc); },
        E::Record(fs) => for (_,v) in fs { free_vars(v, params, acc); },
        E::Unary{ e, .. } => free_vars(e, params, acc),
        E::Bin{ a, b, .. } => { free_vars(a, params, acc); free_vars(b, params, acc); }
        E::Call{ callee, args } => { free_vars(callee, params, acc); for a in args { free_vars(a, params, acc); } }
        E::Index{ target, index } => { free_vars(target, params, acc); free_vars(index, params, acc); }
        E::Field{ target, .. } => free_vars(target, params, acc),
        E::If{ c, a, b } => { free_vars(c, params, acc); free_vars(a, params, acc); free_vars(b, params, acc); }
        E::While{ c, body } => { free_vars(c, params, acc); free_vars(body, params, acc); }
        E::For{ iter, body, .. } => { free_vars(iter, params, acc); free_vars(body, params, acc); }
        E::ForKV{ iter, body, .. } => { free_vars(iter, params, acc); free_vars(body, params, acc); }
        E::Assign{ e, .. } => free_vars(e, params, acc),
        E::AssignLv{ e, .. } => free_vars(e, params, acc),
        E::AssignPat{ e, .. } => free_vars(e, params, acc),
        E::ClassDef{ fields, methods, smethods, .. } => {
            for (_,v) in fields { free_vars(v, params, acc); }
            for (_,ps,b) in methods { let mut p2=params.to_vec(); p2.extend(ps.clone()); free_vars(b, &p2, acc); }
            for (_,ps,b) in smethods { let mut p2=params.to_vec(); p2.extend(ps.clone()); free_vars(b, &p2, acc); }
        }
        E::New{ overrides, .. } => { for (_,v) in overrides { free_vars(v, params, acc); } }
        E::ImportMod{..} | E::ImportFrom{..} | E::Export(..) | E::Trait{..} | E::Impl{..} | E::Match{..} => {}
        _ => {}
    }
}
fn compile_lambda(out: &mut Chunk, params: &[String], body: &Expr) {
    let fname = format!("lambda${}", out.funcs.len());
    let func = compile_function(fname.clone(), params.to_vec(), body);
    out.funcs.push(func);
    // Build env: determine free vars, load them into a record {k:v,...}
    let mut set = std::collections::BTreeSet::<String>::new();
    free_vars(body, params, &mut set);
    for k in &set { out.ops.push(Op::PushStr(k.clone())); out.ops.push(Op::LoadGlobal(k.clone())); }
    out.ops.push(Op::MakeRecord(set.len()));
    out.ops.push(Op::CreateClosure(fname));
}

fn peephole_ops(ops: &mut Vec<Op>){
    let mut outv: Vec<Op> = Vec::with_capacity(ops.len());
    let mut i=0usize;
    while i<ops.len(){
        if i+1<ops.len(){
            match (&ops[i], &ops[i+1]){
                (Op::Dup, Op::Pop) => { i+=2; continue; }
                (Op::Jump(t), _) if *t==i+1 => { i+=1; continue; }
                _=>{}
            }
        }
        outv.push(ops[i].clone());
        i+=1;
    }
    *ops = outv;
}
pub fn compile_expr(e: &Expr, out: &mut Chunk) {
    use crate::runtime::Expr::*;
    match e {
        Expr::Lam{ params, body } => {
            compile_lambda(out, params, body);
        }
        Expr::Assign{name, e} => {
            if let Expr::Lam{params, body} = &**e {
                compile_lambda(out, params, body);
                out.ops.push(Op::StoreGlobal(name.clone()));
            } else {
                compile_expr(e, out);
                out.ops.push(Op::StoreGlobal(name.clone()));
            }
        }
        Expr::Call{ callee, args } => { /* v8_calls */
            // print special-case
            if let Expr::Var(p) = &**callee {
                if p == "print" {
                    if let Some(arg0) = args.get(0) { compile_expr(arg0, out); }
                    out.ops.push(Op::Print);
                } else {
                    // generic: call value
                    compile_expr(callee, out);
                    for a in args { compile_expr(a, out); }
                    out.ops.push(Op::CallValue(args.len()));
                }
            } else if let Expr::Field{ target, name } = &**callee {
                // keep previous method/static lowering (already inserted in v6)
                if let Expr::Var(cls) = &**target {
                    if cls.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) {
                        for a in args { compile_expr(a, out); }
                        out.ops.push(Op::CallStatic(format!("{}::{}", cls, name), args.len()));
                    } else {
                        compile_expr(target, out);
                        for a in args { compile_expr(a, out); }
                        out.ops.push(Op::CallMethod(name.clone(), args.len()));
                    }
                } else {
                    compile_expr(target, out);
                    for a in args { compile_expr(a, out); }
                    out.ops.push(Op::CallMethod(name.clone(), args.len()));
                }
            } else {
                compile_expr(callee, out);
                for a in args { compile_expr(a, out); }
                out.ops.push(Op::CallValue(args.len()));
            }
        }
        Expr::Index{ target, index } => { compile_expr(target, out); compile_expr(index, out); out.ops.push(Op::Index); }
        
        Expr::Num(n) => { let i=intern_const_num(out, *n); out.ops.push(Op::PushConst(i)); }
        Expr::Str(s) => { let i=intern_const_str(out, s); out.ops.push(Op::PushConst(i)); }
        Expr::Call{ callee, args } => { /* v6_call */
            if let Expr::Field{ target, name } = &**callee {
                // method or static call
                if let Expr::Var(cls) = &**target { if cls.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) {
                    for a in args { compile_expr(a, out); }
                    out.ops.push(Op::CallStatic(format!("{}::{}", cls, name), args.len()));
                } else {
                    compile_expr(target, out);
                    for a in args { compile_expr(a, out); }
                    out.ops.push(Op::CallMethod(name.clone(), args.len()));
                }
            } else {
                // best-effort: handle print(...) as before (if existing path), else ignore
                // try to compile callee then args and rely on runtime 'print' lowering if present
                compile_expr(callee, out);
                for a in args { compile_expr(a, out); }
                // If your original compiler lowered print specially, it will still apply.
            }
        }
        Expr::ClassDef{ name, fields, methods, parent, smethods, .. } => { /* v5_classdef */
            // defaults = parent.defaults (clone) or {} then apply own fields
            if let Some(pn) = parent {
                // save parent name for inheritance lookup
                out.ops.push(Op::PushStr(pn.clone()));
                out.ops.push(Op::StoreGlobal(format!("{}$parent", name)));
                // defaults <- clone(Parent$defaults)
                out.ops.push(Op::LoadGlobal(format!("{}$defaults", pn)));
                out.ops.push(Op::CloneRecord);
            } else {
                // start from empty record
                // (emit 0-key record)
                out.ops.push(Op::MakeRecord(0));
            }
            // apply own field defaults
            for (k,v) in fields {
                out.ops.push(Op::Dup);
                compile_expr(v, out);
                out.ops.push(Op::SetField(k.clone()));
                out.ops.push(Op::Pop);
            }
            // store merged defaults
            out.ops.push(Op::StoreGlobal(format!("{}$defaults", name)));
            // compile instance methods to function pool as Name.method
            for (mname, params, body) in methods {
                let fname = format!("{}.{}", name, mname);
                let func = compile_function(fname, params.clone(), body);
                out.funcs.push(func);
            }
            // compile static methods as Name::method (stockées pour la suite)
            for (mname, params, body) in smethods {
                let fname = format!("{}::{}", name, mname);
                let func = compile_function(fname, params.clone(), body);
                out.funcs.push(func);
            }
        }
            // 1) emit defaults record into global "Name$defaults"
            for (k,v) in fields { out.ops.push(Op::PushStr(k.clone())); compile_expr(v, out); }
            out.ops.push(Op::MakeRecord(fields.len()));
            out.ops.push(Op::StoreGlobal(format!("{}$defaults", name)));
            // 2) compile methods to function pool
            for (mname, params, body) in methods {
                let fname = format!("{}.{}", name, mname);
                let func = compile_function(fname, params.clone(), body);
                out.funcs.push(func);
            }
        }
        Expr::New{ name, overrides } => {
            // clone defaults -> instance; tag with __class
            out.ops.push(Op::LoadGlobal(format!("{}$defaults", name)));
            out.ops.push(Op::CloneRecord);
            out.ops.push(Op::Dup);
            out.ops.push(Op::PushStr(name.clone()));
            out.ops.push(Op::SetField("__class".into()));
            out.ops.push(Op::Pop);
            // apply overrides: for each k:v => dup; v; setfield(k); pop
            for (k,v) in overrides {
                out.ops.push(Op::Dup);
                compile_expr(v, out);
                out.ops.push(Op::SetField(k.clone()));
                out.ops.push(Op::Pop);
            }
            // init call if available at runtime via CallMethod("init",0)
            out.ops.push(Op::Dup);
            out.ops.push(Op::CallMethod("init".into(), 0));
            out.ops.push(Op::Pop);
        }
        Expr::Var(name) => { out.ops.push(Op::LoadGlobal(name.clone())); }
        Expr::Assign{name, e} => { compile_expr(e, out); out.ops.push(Op::StoreGlobal(name.clone())); }
        Expr::Record(fs) => {
            // push k1, v1, k2, v2 ...
            for (k,v) in fs { out.ops.push(Op::PushStr(k.clone())); compile_expr(v, out); }
            out.ops.push(Op::MakeRecord(fs.len()));
        }
        Expr::Field{target, name} => { compile_expr(target, out); out.ops.push(Op::GetField(name.clone())); }
        Expr::AssignLv{ lv, e } => {
            if let crate::runtime::LValue::Field{ base, name } = lv {
                out.ops.push(Op::LoadGlobal(base.clone()));
                compile_expr(e, out);
                out.ops.push(Op::SetField(name.clone()));
            }
        }
        Expr::While{ c, body } => {
            let start = out.ops.len();
            compile_expr(c, out);
            let jf_pos = out.ops.len();
            out.ops.push(Op::JumpIfFalse(usize::MAX));
            compile_expr(body, out);
            out.ops.push(Op::Jump(start));
            let end = out.ops.len();
            if let Op::JumpIfFalse(ref mut tgt) = out.ops[jf_pos] { *tgt = end; }
        }
        Expr::If{ c, a, b } => {
            // c
            compile_expr(c, out);
            // patch target after then
            let jmp_false_pos = out.ops.len();
            out.ops.push(Op::JumpIfFalse(usize::MAX));
            // then
            compile_expr(a, out);
            // jump to end
            let jmp_end_pos = out.ops.len();
            out.ops.push(Op::Jump(usize::MAX));
            // patch JumpIfFalse to here
            let after_then = out.ops.len();
            if let Op::JumpIfFalse(ref mut tgt) = out.ops[jmp_false_pos] { *tgt = after_then; }
            // else
            compile_expr(b, out);
            // patch end
            let after_else = out.ops.len();
            if let Op::Jump(ref mut tgt) = out.ops[jmp_end_pos] { *tgt = after_else; }
        }
        Expr::Prog(xs) => { for x in xs { compile_expr(x, out); } peephole_ops(&mut out.ops); }
        Expr::Str(s) => out.ops.push(Op::PushStr(s.clone())) ,
        Num(n) => out.ops.push(Op::PushNum(*n)),
        Bin{op, a, b} => {
            compile_expr(a, out);
            compile_expr(b, out);
            match op.as_str() {
                "+" => out.ops.push(Op::Add),
                "-" => out.ops.push(Op::Sub),
                "*" => out.ops.push(Op::Mul),
                "/" => out.ops.push(Op::Div),
                _ => {}
            }
        }
        Call{callee, args} => {
            if let Var(name) = &**callee {
                if name == "print" && args.len()==1 {
                    compile_expr(&args[0], out);
                    out.ops.push(Op::Print);
                }
            }
        }
        _ => {}
    }
}

pub fn compile_file(path: &Path) -> Result<Chunk> {
    let src = util::read(path)?;
    let toks = tokenize(&src);
    let mut p = Parser::new(toks);
    let prog = p.parse_program()?;
    let mut chunk = Chunk{ ops: vec![] };
    compile_expr(&prog, &mut chunk);
    Ok(chunk)
}

impl Default for Chunk { fn default()->Self{ Self{ ops:vec![], funcs:vec![], consts:vec![] } } } } }
