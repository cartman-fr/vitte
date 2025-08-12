
use std::collections::HashMap;
use vitte_ast::Expr;

#[derive(Debug, Clone)]
pub enum Val { Num(f64), Str(String), List(Vec<Val>), Unit }
impl Val {
    pub fn truthy(&self)->bool{
        match self { Val::Num(n)=>*n!=0.0, Val::Str(s)=>!s.is_empty(), Val::List(v)=>!v.is_empty(), Val::Unit=>false }
    }
}
pub type Env = HashMap<String, Val>;

pub fn eval_capture(prog:&Expr)->Vec<String>{
    let mut out=vec![];
    let mut env:Env=HashMap::new();
    fn eval(env:&mut Env,e:&Expr,out:&mut Vec<String>)->Val{
        match e{
            Expr::Num(n)=>Val::Num(*n),
            Expr::Str(s)=>Val::Str(s.clone()),
            Expr::Var(x)=>env.get(x).cloned().unwrap_or(Val::Unit),
            Expr::List(xs)=>Val::List(xs.iter().map(|x|eval(env,x,out)).collect()),
            Expr::Tuple(xs)=>Val::List(xs.iter().map(|x|eval(env,x,out)).collect()),
            Expr::Unary{op:'-',a}=>match eval(env,a,out){ Val::Num(n)=>Val::Num(-n), _=>Val::Unit },
            Expr::Bin{op,a,b}=>{
                let A=eval(env,a,out); let B=eval(env,b,out);
                match (A,B,op){ (Val::Num(x),Val::Num(y),'+')=>Val::Num(x+y),
                                 (Val::Num(x),Val::Num(y),'-')=>Val::Num(x-y),
                                 (Val::Num(x),Val::Num(y),'*')=>Val::Num(x*y),
                                 (Val::Num(x),Val::Num(y),'/')=>Val::Num(x/y),
                                 _=>Val::Unit }
            }
            Expr::Call{callee,args}=>{
                if let Expr::Var(name)=&**callee{
                    if name=="print"{
                        let v=eval(env,&args[0],out);
                        out.push(render(v));
                        return Val::Unit;
                    } else if name=="len"{
                        let v=eval(env,&args[0],out);
                        let n = match v { Val::List(xs)=>xs.len() as f64, Val::Str(s)=>s.len() as f64, _=>0.0 };
                        return Val::Num(n);
                    } else if name=="push"{
                        let mut v=match eval(env,&args[0],out){ Val::List(xs)=>xs, _=>vec![] };
                        let y=eval(env,&args[1],out);
                        v.push(y); return Val::List(v);
                    }
                }
                Val::Unit
            }
            Expr::If{c,a,b}=>{ let cv=eval(env,c,out); if cv.truthy(){ eval(env,a,out) } else { eval(env,b,out) } }
            Expr::Assign{name,e}=>{ let v=eval(env,e,out); env.insert(name.clone(), v.clone()); v }
            Expr::Prog(stmts)=>{ let mut last=Val::Unit; for s in stmts { last=eval(env,s,out); } last }
        }
    }
    let _ = eval(&mut env, prog, &mut out);
    out
}

pub fn render(v:Val)->String{
    match v{
        Val::Num(n)=>{ if (n - (n as i64 as f64)).abs()<1e-9 { format!("{}", n as i64) } else { format!("{}", n) } }
        Val::Str(s)=>s,
        Val::List(xs)=>{ let inner:Vec<String>=xs.into_iter().map(render).collect(); format!("[{}]", inner.join(", ")) }
        Val::Unit=>"()".into(),
    }
}
