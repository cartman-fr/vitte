use vitte_ast::Expr;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    PushI64=1, PushStr=2,
    Add=3, Sub=4, Mul=5, Div=6,
    Print=7, MakeList=8, Len=9,
    LoadG=10, StoreG=11,
    Jmp=12, Jz=13,
    Call=14, Ret=15,
    Halt=16,
}

#[derive(Debug, Clone)]
pub struct Function { pub name: String, pub arity: u32, pub code: Vec<u8> }

#[derive(Debug, Clone)]
pub struct Chunk { pub code: Vec<u8>, pub const_i64: Vec<i64>, pub const_str: Vec<String>, pub funs: Vec<Function> }
impl Chunk {
    pub fn new()->Self{ Self{ code:vec![], const_i64:vec![], const_str:vec![], funs:vec![] } }
    pub fn emit(&mut self, op:Op){ self.code.push(op as u8); }
    pub fn emit_u32(&mut self, x:u32){ self.code.extend_from_slice(&x.to_le_bytes()); }
    pub fn add_i64(&mut self, v:i64)->u32{ let id=self.const_i64.len() as u32; self.const_i64.push(v); id }
    pub fn add_str(&mut self, s:String)->u32{ let id=self.const_str.len() as u32; self.const_str.push(s); id }
}

pub fn compile(e:&Expr)->Chunk{
    let mut c=Chunk::new();
    compile_into(&mut c, e);
    c.emit(Op::Halt);
    c
}

fn compile_into(c:&mut Chunk, e:&Expr){
    match e {
        Expr::Num(n)=>{ let id=c.add_i64(*n as i64); c.emit(Op::PushI64); c.emit_u32(id); }
        Expr::Str(s)=>{ let id=c.add_str(s.clone()); c.emit(Op::PushStr); c.emit_u32(id); }
        Expr::Var(name)=>{ let id=c.add_str(name.clone()); c.emit(Op::LoadG); c.emit_u32(id); }
        Expr::Assign{name, e}=>{ compile_into(c, e); let id=c.add_str(name.clone()); c.emit(Op::StoreG); c.emit_u32(id); }
        Expr::Bin{op,a,b}=>{ compile_into(c,a); compile_into(c,b); match op{ '+'=>c.emit(Op::Add), '-'=>c.emit(Op::Sub), '*'=>c.emit(Op::Mul), '/'=>c.emit(Op::Div), _=>{} } }
        Expr::Call{callee,args}=>{
            if let Expr::Var(fun) = &**callee {
                if fun=="print" && args.len()==1 { compile_into(c, &args[0]); c.emit(Op::Print); return; }
                if fun=="len" && args.len()==1 { compile_into(c, &args[0]); c.emit(Op::Len); return; }
                // otherwise: compile args then Call <name> <arity>
                for a in args { compile_into(c, a); }
                let id = c.add_str(fun.clone());
                c.emit(Op::Call); c.emit_u32(id); c.emit_u32(args.len() as u32);
                return;
            }
        }
        Expr::List(xs)=>{ for x in xs { compile_into(c,x); } c.emit(Op::MakeList); c.emit_u32(xs.len() as u32); }
        Expr::If{c:a,a:b,b:d}=>{
            compile_into(c, a);
            c.emit(Op::Jz); let pos_jz=c.code.len(); c.emit_u32(0);
            compile_into(c, b); c.emit(Op::Jmp); let pos_jmp=c.code.len(); c.emit_u32(0);
            let here = c.code.len() as u32; c.code[pos_jz..pos_jz+4].copy_from_slice(&here.to_le_bytes());
            compile_into(c, d);
            let end = c.code.len() as u32; c.code[pos_jmp..pos_jmp+4].copy_from_slice(&end.to_le_bytes());
        }
        Expr::FnDef{name, params, body} => {
            let mut fcode = vec![];
            let mut fc = Chunk{ code: vec![], const_i64: vec![], const_str: vec![], funs: vec![] };
            // redirect compile into local vec, but reuse constants of main chunk to keep it simple:
            // We'll compile body by appending into fcode via a small helper that proxies to the same const pools.
            fn comp_body(code:&mut Vec<u8>, c:&mut Chunk, e:&Expr){
                match e {
                    Expr::Num(n)=>{ let id=c.add_i64(*n as i64); code.push(Op::PushI64 as u8); code.extend_from_slice(&id.to_le_bytes()); }
                    Expr::Str(s)=>{ let id=c.add_str(s.clone()); code.push(Op::PushStr as u8); code.extend_from_slice(&id.to_le_bytes()); }
                    Expr::Var(name)=>{ let id=c.add_str(name.clone()); code.push(Op::LoadG as u8); code.extend_from_slice(&id.to_le_bytes()); }
                    Expr::Assign{name, e}=>{ comp_body(code, c, e); let id=c.add_str(name.clone()); code.push(Op::StoreG as u8); code.extend_from_slice(&id.to_le_bytes()); }
                    Expr::Bin{op,a,b}=>{ comp_body(code,c,a); comp_body(code,c,b);
                        code.push(match op{ '+'=>Op::Add, '-'=>Op::Sub, '*'=>Op::Mul, '/'=>Op::Div, _=>Op::Add } as u8); }
                    Expr::Call{callee,args}=>{
                        if let Expr::Var(fun) = &**callee {
                            if fun=="print" && args.len()==1 { comp_body(code,c,&args[0]); code.push(Op::Print as u8); return; }
                            if fun=="len" && args.len()==1 { comp_body(code,c,&args[0]); code.push(Op::Len as u8); return; }
                            for a in args { comp_body(code,c,a); }
                            let id = c.add_str(fun.clone());
                            code.push(Op::Call as u8); code.extend_from_slice(&id.to_le_bytes()); code.extend_from_slice(&(args.len() as u32).to_le_bytes());
                            return;
                        }
                    }
                    Expr::List(xs)=>{ for x in xs { comp_body(code,c,x); } code.push(Op::MakeList as u8); code.extend_from_slice(&(xs.len() as u32).to_le_bytes()); }
                    Expr::If{c:a,a:b,b:d}=>{
                        comp_body(code,c,a);
                        code.push(Op::Jz as u8); let pos_jz = code.len(); code.extend_from_slice(&0u32.to_le_bytes());
                        comp_body(code,c,b); code.push(Op::Jmp as u8); let pos_jmp = code.len(); code.extend_from_slice(&0u32.to_le_bytes());
                        let here = code.len() as u32; code[pos_jz..pos_jz+4].copy_from_slice(&here.to_le_bytes());
                        comp_body(code,c,d);
                        let end = code.len() as u32; code[pos_jmp..pos_jmp+4].copy_from_slice(&end.to_le_bytes());
                    }
                    _=>{}
                }
            }
            comp_body(&mut fcode, c, body);
            fcode.push(Op::Ret as u8);
            c.funs.push(Function{ name: name.clone(), arity: params.len() as u32, code: fcode });
        }
        Expr::Lambda{..}|Expr::Tuple(_)|Expr::Record(_)|Expr::Import(_)=>{ /* not implemented */ }
        Expr::Prog(xs)=> for x in xs { compile_into(c,x); }
    }
}
