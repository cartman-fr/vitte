use std::collections::HashMap;
use vitte_ast::Expr;
use vitte_ty::Ty;
#[derive(Default)] pub struct Infer{}
pub fn infer_expr(_i:&mut Infer,_env:&mut HashMap<String,Ty>,_e:&Expr)->Ty{ Ty::Unknown }
