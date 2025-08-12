use std::collections::HashMap;
use vitte_bytecode::{Chunk, Op};

#[derive(Debug, Clone)]
pub enum Val{ I64(i64), Str(String), List(Vec<Val>), Unit }

pub struct VM{ pub stack: Vec<Val>, pub ip: usize, pub globals: HashMap<String, Val> }
impl VM{
    pub fn new()->Self{ Self{ stack:vec![], ip:0, globals:HashMap::new() } }
    pub fn run(&mut self, c:&Chunk){
        let code=&c.code;
        let mut read_u32 = |ip:&mut usize|{ let mut b=[0u8;4]; b.copy_from_slice(&code[*ip..*ip+4]); *ip+=4; u32::from_le_bytes(b) };
        while self.ip < code.len(){
            let byte = code[self.ip]; self.ip+=1;
            let op = match byte {
                x if x == Op::PushI64 as u8 => Op::PushI64,
                x if x == Op::PushStr as u8 => Op::PushStr,
                x if x == Op::Add as u8 => Op::Add,
                x if x == Op::Sub as u8 => Op::Sub,
                x if x == Op::Mul as u8 => Op::Mul,
                x if x == Op::Div as u8 => Op::Div,
                x if x == Op::Print as u8 => Op::Print,
                x if x == Op::MakeList as u8 => Op::MakeList,
                x if x == Op::Len as u8 => Op::Len,
                x if x == Op::LoadG as u8 => Op::LoadG,
                x if x == Op::StoreG as u8 => Op::StoreG,
                x if x == Op::Jmp as u8 => Op::Jmp,
                x if x == Op::Jz as u8 => Op::Jz,
                x if x == Op::Halt as u8 => Op::Halt,
                _ => Op::Halt,
            };
            match op {
                Op::PushI64 => { let id=read_u32(&mut self.ip); self.stack.push(Val::I64(c.const_i64[id as usize])); }
                Op::PushStr => { let id=read_u32(&mut self.ip); self.stack.push(Val::Str(c.const_str[id as usize].clone())); }
                Op::Add => bin(self, |a,b| a+b),
                Op::Sub => bin(self, |a,b| a-b),
                Op::Mul => bin(self, |a,b| a*b),
                Op::Div => bin(self, |a,b| a/b),
                Op::Print => { if let Some(v)=self.stack.pop(){ println!("{}", render(v)); } }
                Op::MakeList => { let n=read_u32(&mut self.ip) as usize; let mut xs=Vec::with_capacity(n); for _ in 0..n { xs.push(self.stack.pop().unwrap()); } xs.reverse(); self.stack.push(Val::List(xs)); }
                Op::Len => { if let Some(v)=self.stack.pop(){ let n=match v { Val::List(xs)=>xs.len() as i64, Val::Str(s)=>s.len() as i64, _=>0 }; self.stack.push(Val::I64(n)); } }
                Op::LoadG => { let id=read_u32(&mut self.ip); let k=c.const_str[id as usize].clone(); let v=self.globals.get(&k).cloned().unwrap_or(Val::Unit); self.stack.push(v); }
                Op::StoreG => { let id=read_u32(&mut self.ip); let k=c.const_str[id as usize].clone(); let v=self.stack.pop().unwrap_or(Val::Unit); self.globals.insert(k, v); }
                Op::Jmp => { let to = read_u32(&mut self.ip) as usize; self.ip = to; }
                Op::Jz  => { let to = read_u32(&mut self.ip) as usize; let v = self.stack.pop().unwrap(); let z = match v { Val::I64(n)=>n==0, Val::Str(s)=>s.is_empty(), Val::List(xs)=>xs.is_empty(), Val::Unit=>true }; if z { self.ip = to; } }
                Op::Halt => break,
                _ => break,
            }
        }
    }
}
fn bin(vm:&mut VM, f:fn(i64,i64)->i64){
    let b = match vm.stack.pop().unwrap(){ Val::I64(n)=>n, _=>0 };
    let a = match vm.stack.pop().unwrap(){ Val::I64(n)=>n, _=>0 };
    vm.stack.push(Val::I64(f(a,b)));
}
pub fn render(v:Val)->String{
    match v{ Val::I64(n)=>format!("{}", n), Val::Str(s)=>s, Val::List(xs)=>format!("[{}]", xs.into_iter().map(render).collect::<Vec<_>>().join(", ")), Val::Unit=>"()".into() }
}
