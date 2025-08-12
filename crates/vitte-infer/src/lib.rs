use std::collections::HashMap;
use vitte_ast::Expr;
use vitte_ty::Ty;
use vitte_std::prelude;

#[derive(Default)]
pub struct Infer { pub next:u32, pub env: HashMap<String, Ty> }

impl Infer {
    pub fn new()->Self { let mut i=Infer::default(); for (k,v) in prelude().fns { i.env.insert(k, v.ty); } i }
    fn fresh(&mut self)->Ty { let v=self.next; self.next+=1; Ty::Var(v) }
}

pub fn infer_expr(i:&mut Infer, env:&mut HashMap<String, Ty>, e:&Expr)->Ty{
    match e{
        Expr::Num(_)=>Ty::Int,
        Expr::Str(_)=>Ty::Str,
        Expr::Var(x)=>env.get(x).cloned().or_else(||i.env.get(x).cloned()).unwrap_or(Ty::Unknown),
        Expr::List(xs)=>{ let _elt = i.fresh(); let _ = xs; Ty::List(Box::new(Ty::Unknown)) }
        Expr::Tuple(_)=>Ty::Unknown,
        Expr::Record(_)=>Ty::Unknown,
        Expr::Unary{..}|Expr::Bin{..}=>Ty::Int,
        Expr::Call{..}=>Ty::Unknown,
        Expr::If{a,..}=>infer_expr(i, env, a),
        Expr::Lambda{params, body}=>{ let mut _local=env.clone(); let _ = (params, body); Ty::Unknown }
        Expr::Assign{name,e2}=>{ let t=infer_expr(i, env, e2); env.insert(name.clone(), t.clone()); t }
        Expr::FnDef{name, params, body}=>{ let _=(params, body); env.insert(name.clone(), Ty::Unknown); Ty::Unknown }
        Expr::Import(_)=>Ty::Unknown,
        Expr::Prog(xs)=>{ let mut last=Ty::Unknown; for x in xs { last=infer_expr(i, env, x); } last }
    }
}
