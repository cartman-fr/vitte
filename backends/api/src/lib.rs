//! Vitte Backend API v3 — engineer-grade
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarTy { I1, I8, I32, I64, F32, F64, U8 }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty { Unit, Bool, Scalar(ScalarTy), Ptr(Box<Ty>) }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Val(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Block(pub u32);

#[derive(Debug, Clone)]
pub enum MirInst {
    // Const / arith
    ConstI64 { dst: Val, imm: i64 },
    ConstI32 { dst: Val, imm: i32 },
    IAdd { dst: Val, a: Val, b: Val },
    ISub { dst: Val, a: Val, b: Val },
    IMul { dst: Val, a: Val, b: Val },
    IDiv { dst: Val, a: Val, b: Val },
    // Memory
    Alloca { dst: Val, size: u32, align: u32 },
    Load { dst: Val, ptr: Val },
    Store { src: Val, ptr: Val },
    // Control-flow
    Br { target: Block },
    CondBr { cond: Val, then_blk: Block, else_blk: Block },
    Ret { val: Option<Val> },
    // Calls / extern
    Call { dst: Option<Val>, name: String, args: Vec<Val> },
    // Globals
    GlobalStr { dst: Val, symbol: String }, // returns pointer to NUL-terminated data
    // Helper
    Print { arg: Val },
}

#[derive(Debug, Clone)]
pub struct MirBlock { pub id: Block, pub insts: Vec<MirInst> }

#[derive(Debug, Clone)]
pub struct MirFnAttr { pub inline: bool, pub cold: bool }
impl Default for MirFnAttr {
    fn default() -> Self { Self{ inline:false, cold:false } }
}

#[derive(Debug, Clone)]
pub struct MirFn {
    pub name: String,
    pub ret: Ty,
    pub params: Vec<Ty],
    pub blocks: Vec<MirBlock>,
    pub regs: u32,
    pub attrs: MirFnAttr,
}

#[derive(Debug, Clone)]
pub struct Target { pub triple: String, pub pointer_width: u8, pub endian: Endianness }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Endianness { Little, Big }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum CallConv { SystemV, Windows, Wasm }

#[derive(Debug, Clone)]
pub struct BuildResult { pub artifact: String, pub log: String }

pub trait Backend {
    fn target(&self) -> &Target;
    fn compile_fn(&mut self, func: &MirFn, out_stem: &str) -> std::io::Result<BuildResult>;
    fn link(&mut self, objects: &[String], out_vitx: &str) -> std::io::Result<BuildResult>;
}

pub struct MirBuilder {
    next_val: u32,
    pub cur_block: Block,
    pub blocks: Vec<MirBlock>,
    ret_ty: Ty,
    name: String,
    attrs: MirFnAttr,
}
impl MirBuilder {
    pub fn new(name: &str, ret: Ty) -> Self {
        let entry = Block(0);
        Self { next_val:0, cur_block: entry, blocks: vec![MirBlock{id:entry, insts:vec![]}], ret_ty: ret, name: name.to_string(), attrs: MirFnAttr::default() }
    }
    pub fn reg(&mut self) -> Val { let v=Val(self.next_val); self.next_val+=1; v }
    pub fn block(&mut self) -> Block { let id=Block(self.blocks.len() as u32); self.blocks.push(MirBlock{id, insts:vec![]}); id }
    fn push(&mut self, i: MirInst){ let idx = self.cur_block.0 as usize; self.blocks[idx].insts.push(i); }
    pub fn const_i64(&mut self, imm: i64) -> Val { let v=self.reg(); self.push(MirInst::ConstI64{dst:v, imm}); v }
    pub fn iadd(&mut self, a: Val, b: Val) -> Val { let v=self.reg(); self.push(MirInst::IAdd{dst:v, a, b}); v }
    pub fn alloca(&mut self, size:u32, align:u32)->Val{ let v=self.reg(); self.push(MirInst::Alloca{dst:v,size,align}); v }
    pub fn store(&mut self, src:Val, ptr:Val){ self.push(MirInst::Store{src,ptr}); }
    pub fn load(&mut self, ptr:Val)->Val{ let v=self.reg(); self.push(MirInst::Load{dst:v,ptr}); v }
    pub fn gstr(&mut self, sym:&str)->Val{ let v=self.reg(); self.push(MirInst::GlobalStr{dst:v, symbol:sym.into()}); v }
    pub fn call(&mut self, name:&str, args:Vec<Val>)->Option<Val>{ let v=self.reg(); self.push(MirInst::Call{dst:Some(v),name:name.into(),args}); Some(v) }
    pub fn call_void(&mut self, name:&str, args:Vec<Val>){ self.push(MirInst::Call{dst:None,name:name.into(),args}); }
    pub fn print(&mut self, v:Val){ self.push(MirInst::Print{arg:v}); }
    pub fn br(&mut self, target:Block){ self.push(MirInst::Br{target}); }
    pub fn cbr(&mut self, c:Val, t:Block, e:Block){ self.push(MirInst::CondBr{cond:c, then_blk:t, else_blk:e}); }
    pub fn ret(&mut self, v:Option<Val>){ self.push(MirInst::Ret{val:v}); }
    pub fn finish(self) -> MirFn { MirFn{ name:self.name, ret:self.ret_ty, params:vec![], blocks:self.blocks, regs:self.next_val, attrs:self.attrs } }
}

/// Lightweight diagnostics (string-based)
#[derive(Debug, Clone)]
pub struct Diag { pub level: DiagLevel, pub msg: String }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum DiagLevel { Info, Warn, Error }

pub fn write_c_print(out_stem: &str, message: &str) -> std::io::Result<String> {
    use std::fs;
    let c_path = format!("{out}.c", out=out_stem);
    let msg = message.replace("\"","\\\"");
    let code = format!("#include <stdio.h>\\nint main(){{ puts(\\\"{}\\\"); return 0; }}\\n", msg);
    fs::write(&c_path, code)?;
    Ok(c_path)
}
pub fn cc_build(c_path: &str, out_stem: &str) -> std::io::Result<BuildResult> {
    use std::process::Command;
    let bin = format!("{out}.vitx", out=out_stem);
    let cmd = format!("cc -O2 {c} -o {b} && strip {b}", c=c_path, b=bin);
    let status = Command::new("sh").arg("-lc").arg(cmd).status();
    let ok = status.map(|s| s.success()).unwrap_or(false);
    let log = if ok { format!("built {bin}") } else { format!("left C at {c_path} (no cc?)") };
    Ok(BuildResult{ artifact: (if ok { bin } else { c_path.to_string() }), log })
}
