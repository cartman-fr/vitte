use std::collections::HashMap;
use vitte_ast::Expr;

#[derive(Debug, Default, Clone)]
pub struct Scope { pub symbols: HashMap<String, usize> }

pub fn resolve(prog:&Expr) -> Scope {
    let mut s = Scope::default();
    fn walk(sc:&mut Scope, e:&Expr){
        match e{
            Expr::FnDef{name, ..} => { let id = sc.symbols.len(); sc.symbols.insert(name.clone(), id); }
            Expr::Assign{name, ..} => { let id = sc.symbols.len(); sc.symbols.insert(name.clone(), id); }
            Expr::Prog(xs) => for x in xs { walk(sc,x); },
            _ => {}
        }
    }
    walk(&mut s, prog);
    s
}
