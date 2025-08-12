use vitte_bytecode::{Chunk, Op};

#[derive(Debug, Clone)]
pub enum Val{ I64(i64), Str(String), List(Vec<Val>), Unit }

pub struct VM{ pub stack: Vec<Val>, pub ip: usize }
impl VM{
    pub fn new()->Self{ Self{ stack:vec![], ip:0 } }
    pub fn run(&mut self, c:&Chunk){
        let code=&c.code;
        macro_rules! rd { () => {{ let mut b=[0u8;4]; b.copy_from_slice(&code[self.ip..self.ip+4]); self.ip+=4; u32::from_le_bytes(b) }} }
        while self.ip < code.len(){
            let op = unsafe{ std::mem::transmute::<u8, Op>(code[self.ip]) }; self.ip+=1;
            match op {
                Op::PushI64 => { let id=rd!(); self.stack.push(Val::I64(c.const_i64[id as usize])); }
                Op::PushStr => { let id=rd!(); self.stack.push(Val::Str(c.const_str[id as usize].clone())); }
                Op::Add => bin(self, |a,b| a+b),
                Op::Sub => bin(self, |a,b| a-b),
                Op::Mul => bin(self, |a,b| a*b),
                Op::Div => bin(self, |a,b| a/b),
                Op::Print => { if let Some(v)=self.stack.pop(){ println!("{}", render(v)); } }
                Op::MakeList => { let n=rd!() as usize; let mut xs=Vec::with_capacity(n); for _ in 0..n { xs.push(self.stack.pop().unwrap()); } xs.reverse(); self.stack.push(Val::List(xs)); }
                Op::Len => { if let Some(v)=self.stack.pop(){ let n=match v { Val::List(xs)=>xs.len() as i64, Val::Str(s)=>s.len() as i64, _=>0 }; self.stack.push(Val::I64(n)); } }
                Op::Jmp => { let to = rd!() as usize; self.ip = to; }
                Op::Jz  => { let to = rd!() as usize; let v = self.stack.pop().unwrap(); let z = match v { Val::I64(n)=>n==0, Val::Str(s)=>s.is_empty(), Val::List(xs)=>xs.is_empty(), Val::Unit=>True }; if z { self.ip = to; } }
                Op::Ret | Op::Halt => break,
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
