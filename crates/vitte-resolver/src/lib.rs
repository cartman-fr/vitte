use std::collections::HashMap;
use vitte_ast::Expr;
#[derive(Debug, Default, Clone)] pub struct Scope{ pub symbols: HashMap<String, usize> }
pub fn resolve(e:&Expr)->Scope{ let mut s=Scope::default(); if let Expr::Prog(xs)=e { for x in xs { if let Expr::Assign{name,..}|Expr::FnDef{name,..}=x { let id=s.symbols.len(); s.symbols.insert(name.clone(), id); } } } s }
