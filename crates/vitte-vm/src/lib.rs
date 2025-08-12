use std::collections::HashMap;
use vitte_bytecode::{Chunk, Op, Function};

#[derive(Debug, Clone)]
pub enum Val{ I64(i64), Str(usize), List(usize), Unit }

#[derive(Debug)]
enum Obj { Str(String, bool), List(Vec<Val>, bool) } // (payload, mark)

pub struct Heap{ objs: Vec<Option<Obj>>, free: Vec<usize> }
impl Heap{
    pub fn new()->Self{ Self{ objs:vec![], free:vec![] } }
    fn alloc_str(&mut self, s:String)->usize{
        if let Some(i)=self.free.pop(){ self.objs[i]=Some(Obj::Str(s,false)); return i; }
        self.objs.push(Some(Obj::Str(s,false))); self.objs.len()-1
    }
    fn alloc_list(&mut self, xs:Vec<Val>)->usize{
        if let Some(i)=self.free.pop(){ self.objs[i]=Some(Obj::List(xs,false)); return i; }
        self.objs.push(Some(Obj::List(xs,false))); self.objs.len()-1
    }
    fn mark_val(&mut self, v:&Val){
        match v{
            Val::Str(h)|Val::List(h)=> self.mark_obj(*h),
            _=>{}
        }
    }
    fn mark_obj(&mut self, h:usize){
        if let Some(Some(obj)) = self.objs.get_mut(h){
            match obj{
                Obj::Str(_, mark) => { if *mark { return; } *mark=true; }
                Obj::List(xs, mark) => {
                    if *mark { return; } *mark=true;
                    for v in xs.iter(){ self.mark_val(v); }
                }
            }
        }
    }
    fn sweep(&mut self){
        for (i,slot) in self.objs.iter_mut().enumerate(){
            if let Some(obj) = slot {
                let marked = match obj { Obj::Str(_,m)|Obj::List(_,m) => *m };
                if marked{
                    match obj { Obj::Str(_,m)|Obj::List(_,m) => *m = false; }
                } else {
                    *slot = None;
                    self.free.push(i);
                }
            }
        }
    }
}

struct Frame<'a>{ code: &'a [u8], ip: usize, stack_base: usize }
pub struct VM<'a>{
    pub stack: Vec<Val>,
    pub globals: HashMap<String, Val>,
    pub heap: Heap,
    funs: Vec<&'a Function>,
    frames: Vec<Frame<'a>>,
}

impl<'a> VM<'a>{
    pub fn new()->Self{ Self{ stack:vec![], globals:HashMap::new(), heap:Heap::new(), funs:vec![], frames:vec![] } }
    fn gc(&mut self){
        // mark roots: stack + globals
        for v in self.stack.iter(){ self.heap.mark_val(v); }
        for v in self.globals.values(){ self.heap.mark_val(v); }
        self.heap.sweep();
    }
    pub fn run(&mut self, c:&'a Chunk){
        self.funs = c.funs.iter().collect();
        let mut code:&[u8] = &c.code;
        let mut ip:usize = 0;
        let mut rd = |code:&[u8], ip:&mut usize|{ let mut b=[0u8;4]; b.copy_from_slice(&code[*ip..*ip+4]); *ip+=4; u32::from_le_bytes(b) };
        while ip < code.len(){
            let byte = code[ip]; ip+=1;
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
                x if x == Op::Call as u8 => Op::Call,
                x if x == Op::Ret as u8 => Op::Ret,
                x if x == Op::Halt as u8 => Op::Halt,
                _ => Op::Halt,
            };
            match op {
                Op::PushI64 => { let id=rd(code,&mut ip); self.stack.push(Val::I64(c.const_i64[id as usize])); }
                Op::PushStr => { let id=rd(code,&mut ip); let s = c.const_str[id as usize].clone(); let h=self.heap.alloc_str(s); self.stack.push(Val::Str(h)); }
                Op::Add => bin(self, |a,b| a+b),
                Op::Sub => bin(self, |a,b| a-b),
                Op::Mul => bin(self, |a,b| a*b),
                Op::Div => bin(self, |a,b| a/b),
                Op::Print => { if let Some(v)=self.stack.pop(){ println!("{}", render(&self.heap, v)); } }
                Op::MakeList => { let n=rd(code,&mut ip) as usize; let mut xs=Vec::with_capacity(n); for _ in 0..n { xs.push(self.stack.pop().unwrap()); } xs.reverse(); let h=self.heap.alloc_list(xs); self.stack.push(Val::List(h)); }
                Op::Len => { if let Some(v)=self.stack.pop(){ let n=match v { Val::List(h)=>{ if let Some(Some(Obj::List(xs,_))) = self.heap.objs.get(h){ xs.len() as i64 } else {0} }, Val::Str(h)=>{ if let Some(Some(Obj::Str(s,_)))=self.heap.objs.get(h){ s.len() as i64 } else {0} }, _=>0 }; self.stack.push(Val::I64(n)); } }
                Op::LoadG => { let id=rd(code,&mut ip); let k=c.const_str[id as usize].clone(); let v=self.globals.get(&k).cloned().unwrap_or(Val::Unit); self.stack.push(v); }
                Op::StoreG => { let id=rd(code,&mut ip); let k=c.const_str[id as usize].clone(); let v=self.stack.pop().unwrap_or(Val::Unit); self.globals.insert(k, v); }
                Op::Jmp => { let to = rd(code,&mut ip) as usize; ip = to; }
                Op::Jz  => { let to = rd(code,&mut ip) as usize; let v = self.stack.pop().unwrap(); let z = is_falsey(&self.heap, &v); if z { ip = to; } }
                Op::Call => {
                    let name_id = rd(code,&mut ip) as usize;
                    let arity = rd(code,&mut ip) as usize;
                    let fname = &c.const_str[name_id];
                    if let Some(fun) = self.funs.iter().find(|f| f.name == *fname) {
                        let frame = Frame{ code, ip, stack_base: self.stack.len() - arity };
                        self.frames.push(frame);
                        code = &fun.code; ip = 0;
                        continue;
                    } else {
                        eprintln!("appel inconnu: {}", fname);
                        // drop args
                        for _ in 0..arity { let _ = self.stack.pop(); }
                        self.stack.push(Val::Unit);
                    }
                }
                Op::Ret => {
                    // result is on stack top (or push Unit if none)
                    let res = self.stack.pop().unwrap_or(Val::Unit);
                    if let Some(frame) = self.frames.pop() {
                        // clean frame args (any locals would have been on stack >= stack_base)
                        while self.stack.len() > frame.stack_base { let _ = self.stack.pop(); }
                        self.stack.push(res);
                        code = frame.code; ip = frame.ip;
                    } else {
                        // ret out of main: halt
                        self.stack.push(res);
                        break;
                    }
                }
                Op::Halt => break,
            }
            // trigger opportunistic GC when stack grows
            if self.stack.len() % 64 == 0 { self.gc(); }
        }
        // final GC sweep
        self.gc();
    }
}

fn is_falsey(heap:&Heap, v:&Val)->bool{
    match v{
        Val::I64(n)=>*n==0,
        Val::Str(h)=>{
            if let Some(Some(Obj::Str(s,_))) = heap.objs.get(*h) { s.is_empty() } else { true }
        }
        Val::List(h)=>{
            if let Some(Some(Obj::List(xs,_))) = heap.objs.get(*h) { xs.is_empty() } else { true }
        }
        Val::Unit=>true
    }
}

fn bin(vm:&mut VM, f:fn(i64,i64)->i64){
    let b = match vm.stack.pop().unwrap(){ Val::I64(n)=>n, _=>0 };
    let a = match vm.stack.pop().unwrap(){ Val::I64(n)=>n, _=>0 };
    vm.stack.push(Val::I64(f(a,b)));
}

pub fn render(heap:&Heap, v:Val)->String{
    match v{
        Val::I64(n)=>format!("{}", n),
        Val::Str(h)=>{
            if let Some(Some(Obj::Str(s,_))) = heap.objs.get(h) { s.clone() } else { "<dangling-str>".into() }
        }
        Val::List(h)=>{
            if let Some(Some(Obj::List(xs,_))) = heap.objs.get(h) {
                let inner:Vec<String>=xs.iter().cloned().map(|v|render(heap,v)).collect();
                format!("[{}]", inner.join(", "))
            } else { "[]".into() }
        }
        Val::Unit=>"()".into(),
    }
}
