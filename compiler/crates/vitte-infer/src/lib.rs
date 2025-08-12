use std::collections::HashMap;
use vitte_hir::H;
use vitte_ty::Ty;
#[derive(Default)] pub struct Infer{}
pub fn infer_prog(_i:&mut Infer,_env:&mut HashMap<String,Ty>,_h:&H)->Ty{ Ty::Unknown }
