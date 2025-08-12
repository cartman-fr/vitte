use vitte_mir::{Mir, Inst};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    PushI64=1, PushStr=2,
    Add=3, Sub=4, Mul=5, Div=6,
    Print=7, MakeList=8, Len=9,
    LoadG=10, StoreG=11,
    Jmp=12, Jz=13,
    Halt=14,
}

#[derive(Debug, Clone)]
pub struct Chunk { pub code: Vec<u8>, pub const_i64: Vec<i64>, pub const_str: Vec<String> }
impl Chunk{
    pub fn new()->Self{ Self{ code:vec![], const_i64:vec![], const_str:vec![] } }
    fn emit(&mut self, op:Op){ self.code.push(op as u8); }
    fn emit_u32(&mut self, x:u32){ self.code.extend_from_slice(&x.to_le_bytes()); }
    fn add_i64(&mut self, v:i64)->u32{ let id=self.const_i64.len() as u32; self.const_i64.push(v); id }
    fn add_str(&mut self, s:String)->u32{ let id=self.const_str.len() as u32; self.const_str.push(s); id }
}

pub fn compile(m:&Mir)->Chunk{
    let mut c=Chunk::new();
    for i in 0..m.code.len() {
        match &m.code[i]{
            Inst::PushI64(n)=>{ let id=c.add_i64(*n); c.emit(Op::PushI64); c.emit_u32(id); }
            Inst::PushStr(s)=>{ let id=c.add_str(s.clone()); c.emit(Op::PushStr); c.emit_u32(id); }
            Inst::MakeList(n)=>{ c.emit(Op::MakeList); c.emit_u32(*n); }
            Inst::Add=>c.emit(Op::Add), Inst::Sub=>c.emit(Op::Sub), Inst::Mul=>c.emit(Op::Mul), Inst::Div=>c.emit(Op::Div),
            Inst::LoadG(k)=>{ let id=c.add_str(k.clone()); c.emit(Op::LoadG); c.emit_u32(id); }
            Inst::StoreG(k)=>{ let id=c.add_str(k.clone()); c.emit(Op::StoreG); c.emit_u32(id); }
            Inst::CallBuiltin(name,1) if name=="print" => c.emit(Op::Print),
            Inst::CallBuiltin(name,1) if name=="len" => c.emit(Op::Len),
            Inst::If{jz_to, jmp_to} => {
                // Need next instruction to know if it's Jz or Jmp placeholder. We'll encode both passes.
                // Encode Jz now, Jmp later when we hit the second placeholder.
                if *jz_to != 0 && (i==0 || !matches!(m.code[i-1], Inst::If{..})) {
                    c.emit(Op::Jz); c.emit_u32(offset_for(c.code.len(), *jz_to, &m));
                } else if *jmp_to != 0 {
                    c.emit(Op::Jmp); c.emit_u32(offset_for(c.code.len(), *jmp_to, &m));
                }
            }
            _=>{}
        }
    }
    c.emit(Op::Halt);
    c
}

// compute absolute bytecode position approximation by walking MIR up to idx
fn offset_for(current_bc_len: usize, target_mir_idx: usize, m:&Mir)->u32{
    // naive: re-encode from start to target to get its byte position
    let mut c = Chunk::new();
    for k in 0..target_mir_idx {
        match &m.code[k]{
            Inst::PushI64(n)=>{ let id=c.add_i64(*n); c.emit(Op::PushI64); c.emit_u32(id); }
            Inst::PushStr(s)=>{ let id=c.add_str(s.clone()); c.emit(Op::PushStr); c.emit_u32(id); }
            Inst::MakeList(n)=>{ c.emit(Op::MakeList); c.emit_u32(*n); }
            Inst::Add|Inst::Sub|Inst::Mul|Inst::Div=>{ c.emit(Op::Add); } // size 1
            Inst::LoadG(k)=>{ let id=c.add_str(k.clone()); c.emit(Op::LoadG); c.emit_u32(id); }
            Inst::StoreG(k)=>{ let id=c.add_str(k.clone()); c.emit(Op::StoreG); c.emit_u32(id); }
            Inst::CallBuiltin(name,1) if name=="print" => c.emit(Op::Print),
            Inst::CallBuiltin(name,1) if name=="len" => c.emit(Op::Len),
            Inst::If{..}=>{ /* placeholders ignored in pass 1 */ }
            _=>{}
        }
    }
    c.code.len() as u32
}
