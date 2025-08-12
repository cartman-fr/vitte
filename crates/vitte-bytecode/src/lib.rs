use vitte_ast::Expr;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Op { PushI64=1, PushStr=2, Add=3, Sub=4, Mul=5, Div=6, Print=7, MakeList=8, Len=9, Jmp=10, Jz=11, Ret=12, Halt=13 }

#[derive(Debug, Clone)]
pub struct Chunk { pub code: Vec<u8>, pub const_i64: Vec<i64>, pub const_str: Vec<String> }
impl Chunk {
    pub fn new()->Self{ Self{ code:vec![], const_i64:vec![], const_str:vec![] } }
    fn emit(&mut self, op:Op){ self.code.push(op as u8); }
    fn emit_u32(&mut self, x:u32){ self.code.extend_from_slice(&x.to_le_bytes()); }
    fn add_i64(&mut self, v:i64)->u32{ let id=self.const_i64.len() as u32; self.const_i64.push(v); id }
    fn add_str(&mut self, s:String)->u32{ let id=self.const_str.len() as u32; self.const_str.push(s); id }
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
        Expr::Bin{op,a,b}=>{
            compile_into(c,a); compile_into(c,b);
            match op{ '+'=>c.emit(Op::Add), '-'=>c.emit(Op::Sub), '*'=>c.emit(Op::Mul), '/'=>c.emit(Op::Div), _=>{} }
        }
        Expr::Call{callee,args}=>{
            if let Expr::Var(fun) = &**callee {
                if fun=="print" && args.len()==1 { compile_into(c, &args[0]); c.emit(Op::Print); return; }
                if fun=="len" && args.len()==1 { compile_into(c, &args[0]); c.emit(Op::Len); return; }
            }
        }
        Expr::List(xs)=>{ for x in xs { compile_into(c,x); } c.emit(Op::MakeList); c.emit_u32(xs.len() as u32); }
        Expr::If{cnd,a,b}=>{
            compile_into(c, cnd);
            c.emit(Op::Jz); let pos_jz=c.code.len(); c.emit_u32(0);
            compile_into(c, a); c.emit(Op::Jmp); let pos_jmp=c.code.len(); c.emit_u32(0);
            let here = c.code.len() as u32; c.code[pos_jz..pos_jz+4].copy_from_slice(&here.to_le_bytes());
            compile_into(c, b);
            let end = c.code.len() as u32; c.code[pos_jmp..pos_jmp+4].copy_from_slice(&end.to_le_bytes());
        }
        Expr::Assign{..}|Expr::Var(_)|Expr::Lambda{..}|Expr::FnDef{..}|Expr::Tuple(_)|Expr::Record(_)|Expr::Import(_)=>{ /* not implemented */ }
        Expr::Prog(xs)=> for x in xs { compile_into(c,x); }
    }
}
