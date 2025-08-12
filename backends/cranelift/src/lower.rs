//! Lower MIR -> pseudo-CLIF (strings format), for debug & tests.
use vitte_backend_api::{MirFn, MirInst, Val};

pub fn to_pseudo_clif(f: &MirFn) -> String {
    let mut s = String::new();
    s.push_str(&format!("function {}() {{\n", f.name));
    for b in &f.blocks {
        s.push_str(&format!("  block{}:\n", b.id.0));
        for i in &b.insts {
            match i {
                MirInst::ConstI64{dst,imm} => s.push_str(&format!("    v{} = iconst.i64 {}\n", dst.0, imm)),
                MirInst::IAdd{dst,a,b} => s.push_str(&format!("    v{} = iadd v{}, v{}\n", dst.0, a.0, b.0)),
                MirInst::ISub{dst,a,b} => s.push_str(&format!("    v{} = isub v{}, v{}\n", dst.0, a.0, b.0)),
                MirInst::IMul{dst,a,b} => s.push_str(&format!("    v{} = imul v{}, v{}\n", dst.0, a.0, b.0)),
                MirInst::IDiv{dst,a,b} => s.push_str(&format!("    v{} = sdiv v{}, v{}\n", dst.0, a.0, b.0)),
                MirInst::Call{dst,name,args} => {
                    let args_str = args.iter().map(|v| format!("v{}", v.0)).collect::<Vec<_>>().join(", ");
                    if let Some(d)=dst { s.push_str(&format!("    v{} = call @{}({})\n", d.0, name, args_str)); }
                    else { s.push_str(&format!("    call @{}({})\n", name, args_str)); }
                }
                MirInst::Print{arg} => s.push_str(&format!("    ; print v{}\n", arg.0)),
                MirInst::Br{target} => s.push_str(&format!("    jump block{}\n", target.0)),
                MirInst::CondBr{cond,then_blk,else_blk} => s.push_str(&format!("    brnz v{}, block{}, block{}\n", cond.0, then_blk.0, else_blk.0)),
                MirInst::Ret{val} => match val { Some(v)=>s.push_str(&format!("    return v{}\n", v.0)), None=>s.push_str("    return\n") },
            }
        }
    }
    s.push_str("}\n");
    s
}
