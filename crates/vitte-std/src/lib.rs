use std::collections::HashMap;
use vitte_ty::{Ty, Scheme};

#[derive(Default, Debug)]
pub struct Prelude { pub fns: HashMap<String, Scheme> }

pub fn prelude() -> Prelude {
    let mut p = Prelude::default();
    p.fns.insert("print".into(), Scheme{ vars: vec![0], ty: Ty::Fn(vec![Ty::Var(0)], Box::new(Ty::Unknown)) });
    p.fns.insert("len".into(),   Scheme{ vars: vec![1], ty: Ty::Fn(vec![Ty::List(Box::new(Ty::Var(1)))], Box::new(Ty::Int)) });
    p.fns.insert("push".into(),  Scheme{ vars: vec![2], ty: Ty::Fn(vec![Ty::List(Box::new(Ty::Var(2))), Ty::Var(2)], Box::new(Ty::List(Box::new(Ty::Var(2))))) });
    p.fns.insert("str".into(),   Scheme{ vars: vec![3], ty: Ty::Fn(vec![Ty::Var(3)], Box::new(Ty::Str)) });
    p
}
