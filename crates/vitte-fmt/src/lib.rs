use vitte_ast::Expr;
pub fn format(e:&Expr)->String{
    match e{
        Expr::Num(n)=>format!("{}", n),
        Expr::Str(s)=>format!(""{}"", s),
        Expr::Var(x)=>x.clone(),
        Expr::List(xs)=>format!("[{}]", xs.iter().map(format).collect::<Vec<_>>().join(", ")),
        Expr::Tuple(xs)=>format!("({})", xs.iter().map(format).collect::<Vec<_>>().join(", ")),
        Expr::Record(fs)=>format!("{{{}}}", fs.iter().map(|(k,v)| format!("{}: {}", k, format(v))).collect::<Vec<_>>().join(", ")),
        Expr::Unary{op,a}=>format!("{}{}", op, format(a)),
        Expr::Bin{op,a,b}=>format!("{} {} {}", format(a), op, format(b)),
        Expr::Call{callee,args}=>format!("{}({})", format(callee), args.iter().map(format).collect::<Vec<_>>().join(", ")),
        Expr::If{c,a,b}=>format!("if {} then {} else {}", format(c), format(a), format(b)),
        Expr::Lambda{params, body}=>format!("({}) -> {}", params.join(", "), format(body)),
        Expr::Assign{name,e}=>format!("{} = {}", name, format(e)),
        Expr::FnDef{name, params, body}=>format!("fn {}({}) = {}", name, params.join(", "), format(body)),
        Expr::Import(p)=>format!("import "{}"", p),
        Expr::Prog(xs)=>xs.iter().map(format).collect::<Vec<_>>().join(";
"),
    }
}
