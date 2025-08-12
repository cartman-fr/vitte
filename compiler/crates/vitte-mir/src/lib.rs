use vitte_hir::H;

#[derive(Debug, Clone)]
pub enum Inst{
    PushI64(i64), PushStr(String), MakeList(u32),
    Add, Sub, Mul, Div,
    LoadG(String), StoreG(String),
    CallBuiltin(String, u32), // print, len
    If{ jz_to: usize, jmp_to: usize },
    // markers for structure; linear stream still
}

#[derive(Debug, Default, Clone)]
pub struct Mir { pub code: Vec<Inst> }

pub fn lower(h:&H)->Mir{
    let mut m=Mir::default();
    emit(&mut m, h);
    m
}
fn emit(m:&mut Mir, h:&H){
    match h{
        H::Num(n)=>m.code.push(Inst::PushI64(*n as i64)),
        H::Str(s)=>m.code.push(Inst::PushStr(s.clone())),
        H::Var(x)=>m.code.push(Inst::LoadG(x.clone())),
        H::Assign{name,e}=>{ emit(m,e); m.code.push(Inst::StoreG(name.clone())); }
        H::Bin{op,a,b}=>{ emit(m,a); emit(m,b); m.code.push(match op{ '+'=>Inst::Add,'-'=>Inst::Sub,'*'=>Inst::Mul,'/'=>Inst::Div,_=>Inst::Add}); }
        H::List(xs)=>{ for x in xs { emit(m,x); } m.code.push(Inst::MakeList(xs.len() as u32)); }
        H::Call{callee,args}=>{
            if let H::Var(name)=&**callee {
                for a in args { emit(m,a); }
                if name=="print" || name=="len" {
                    m.code.push(Inst::CallBuiltin(name.clone(), args.len() as u32));
                }
            }
        }
        H::If{c,a,b}=>{
            emit(m,c);
            let jz_pos = m.code.len(); m.code.push(Inst::If{ jz_to:0, jmp_to:0 });
            emit(m,a);
            let jmp_pos = m.code.len(); m.code.push(Inst::If{ jz_to:0, jmp_to:0 });
            let here = m.code.len();
            if let Inst::If{jz_to,..} = &mut m.code[jz_pos] { *jz_to = here; }
            emit(m,b);
            let end = m.code.len();
            if let Inst::If{jmp_to,..} = &mut m.code[jmp_pos] { *jmp_to = end; }
        }
        H::FnDef{..}=>{} // future: functions in MIR
        H::Prog(xs)=> for x in xs { emit(m,x); }
    }
}
