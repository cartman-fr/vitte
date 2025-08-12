use std::collections::HashMap;
use vitte_hir::H;
#[derive(Debug, Default, Clone)] pub struct Scope{ pub symbols: HashMap<String, usize> }
pub fn resolve(h:&H)->Scope{
    let mut s=Scope::default();
    if let H::Prog(xs)=h { for x in xs {
        match x { H::Assign{name,..}|H::FnDef{name,..} => { let id=s.symbols.len(); s.symbols.insert(name.clone(), id); }, _=>{} }
    } }
    s
}
