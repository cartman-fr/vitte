use vitte_ast::Expr;

#[derive(Debug, Clone)]
pub enum H { Num(f64), Str(String), Var(String), List(Vec<H>), Bin{op:char,a:Box<H>,b:Box<H>},
            Call{callee:Box<H>,args:Vec<H>}, If{c:Box<H>,a:Box<H>,b:Box<H>},
            Assign{name:String,e:Box<H>}, FnDef{name:String,params:Vec<String>,body:Box<H>}, Prog(Vec<H>) }

pub fn lower(e:&Expr)->H{
    match e{
        Expr::Num(n)=>H::Num(*n),
        Expr::Str(s)=>H::Str(s.clone()),
        Expr::Var(x)=>H::Var(x.clone()),
        Expr::List(xs)=>H::List(xs.iter().map(lower).collect()),
        Expr::Bin{op,a,b}=>H::Bin{op:*op,a:Box::new(lower(a)),b:Box::new(lower(b))},
        Expr::Call{callee,args}=>H::Call{ callee:Box::new(lower(callee)), args:args.iter().map(lower).collect() },
        Expr::If{c,a,b}=>H::If{ c:Box::new(lower(c)), a:Box::new(lower(a)), b:Box::new(lower(b)) },
        Expr::Assign{name,e}=>H::Assign{name:name.clone(), e:Box::new(lower(e))},
        Expr::FnDef{name,params,body}=>H::FnDef{name:name.clone(), params:params.clone(), body:Box::new(lower(body))},
        Expr::Prog(xs)=>H::Prog(xs.iter().map(lower).collect()),
        _=>H::Prog(vec![])
    }
}
