
use crate::runtime::{Expr, Pattern, LValue};

pub struct Cfg { pub width: usize }
impl Default for CCfg { fn default()->Self{ Self{ width: 80 } } }
type CCfg = Cfg;

pub fn format(expr: &Expr, cfg: &Cfg) -> String {
    let mut s = String::new();
    fmt_expr(expr, 0, &mut s, cfg);
    s.trim_end().to_string()
}

fn indent(n: usize) -> String { " ".repeat(n) }

fn fmt_block(body: &Expr, ind: usize, out: &mut String, cfg: &Cfg) {
    out.push('\n');
    out.push_str(&indent(ind+2));
    fmt_expr(body, ind+2, out, cfg);
}

fn fmt_list<T>(xs: &[T], out: &mut String, mut f: impl FnMut(&T, &mut String)) {
    for (i, x) in xs.iter().enumerate() {
        if i>0 { out.push_str(", "); }
        f(x, out);
    }
}

fn fmt_pattern(p:&Pattern, out:&mut String){
    match p {
        Pattern::PVar(s) => out.push_str(s),
        Pattern::PTuple(ps) => {
            out.push('(');
            for (i,x) in ps.iter().enumerate(){
                if i>0 { out.push_str(", "); }
                fmt_pattern(x, out);
            }
            out.push(')');
        }
    }
}

fn fmt_expr(e: &Expr, ind: usize, out: &mut String, cfg: &Cfg) {
    use crate::runtime::Expr::*;
    match e {
        Num(n) => { out.push_str(&format!("{}", n)); }
        Str(s) => { out.push('"'); out.push_str(&s); out.push('"'); }
        Bool(b) => { out.push_str(if *b { "true" } else { "false" }); }
        Var(x) => out.push_str(x),
        List(xs) => { out.push('['); fmt_list(xs, out, |x,o| fmt_expr(x, ind, o, cfg)); out.push(']'); }
        Tuple(xs) => { out.push('('); fmt_list(xs, out, |x,o| fmt_expr(x, ind, o, cfg)); out.push(')'); }
        Record(fs) => {
            out.push('{');
            for (i,(k,v)) in fs.iter().enumerate() {
                if i>0 { out.push_str(", "); }
                out.push_str(k); out.push_str(": "); fmt_expr(v, ind, out, cfg);
            }
            out.push('}');
        }
        Unary{op,e} => { out.push_str(op); fmt_expr(e, ind, out, cfg); }
        Bin{op,a,b} => { fmt_expr(a, ind, out, cfg); out.push(' '); out.push_str(op); out.push(' '); fmt_expr(b, ind, out, cfg); }
        Call{callee,args} => { fmt_expr(callee, ind, out, cfg); out.push('('); fmt_list(args, out, |x,o| fmt_expr(x, ind, o, cfg)); out.push(')'); }
        Index{target,index} => { fmt_expr(target, ind, out, cfg); out.push('['); fmt_expr(index, ind, out, cfg); out.push(']'); }
        Field{target,name} => { fmt_expr(target, ind, out, cfg); out.push('.'); out.push_str(name); }
        If{c,a,b} => { out.push_str("if "); fmt_expr(c, ind, out, cfg); out.push_str(" then "); fmt_expr(a, ind, out, cfg); out.push_str(" else "); fmt_expr(b, ind, out, cfg); }
        While{c,body} => { out.push_str("while "); fmt_expr(c, ind, out, cfg); out.push_str(" do"); fmt_block(body, ind, out, cfg); }
        For{var,iter,body} => { out.push_str("for "); out.push_str(var); out.push_str(" in "); fmt_expr(iter, ind, out, cfg); out.push_str(" do"); fmt_block(body, ind, out, cfg); }
        ForKV{k,v,iter,body} => { out.push_str("for ("); out.push_str(k); out.push_str(", "); out.push_str(v); out.push_str(") in "); fmt_expr(iter, ind, out, cfg); out.push_str(" do"); fmt_block(body, ind, out, cfg); }
        Lam{params, body} => { out.push('('); for (i,p) in params.iter().enumerate(){ if i>0 { out.push_str(", "); } out.push_str(p); } out.push_str(") -> "); fmt_expr(body, ind, out, cfg); }
        Assign{name,e} => { out.push_str(name); out.push_str(" = "); fmt_expr(e, ind, out, cfg); }
        AssignLv{lv,e} => {
            match lv {
                LValue::Var(x) => out.push_str(x),
                LValue::Index{base, idx} => { out.push_str(base); out.push('['); fmt_expr(idx, ind, out, cfg); out.push(']'); }
                LValue::Field{base, name} => { out.push_str(base); out.push('.'); out.push_str(name); }
            }
            out.push_str(" = "); fmt_expr(e, ind, out, cfg);
        }
        AssignPat{pat,e} => { fmt_pattern(pat, out); out.push_str(" = "); fmt_expr(e, ind, out, cfg); }
        ClassDef{name, parent, fields, methods, sfields, smethods} => {
            out.push_str("class "); out.push_str(name);
            if let Some(p) = parent { out.push_str(" : "); out.push_str(p); }
            out.push_str(" {");
            for (k,v) in sfields {
                out.push_str("\n"); out.push_str(&" ".repeat(ind+2));
                out.push_str("static "); out.push_str(k); out.push_str(": "); fmt_expr(v, ind+2, out, cfg);
            }
            for (k,v) in fields {
                out.push_str("\n"); out.push_str(&" ".repeat(ind+2));
                out.push_str(k); out.push_str(": "); fmt_expr(v, ind+2, out, cfg);
            }
            for (n,ps,b) in smethods {
                out.push_str("\n"); out.push_str(&" ".repeat(ind+2));
                out.push_str("static "); out.push_str(n); out.push('(');
                for (i,p) in ps.iter().enumerate(){ if i>0 { out.push_str(", "); } out.push_str(p); }
                out.push_str(") -> "); fmt_expr(b, ind+2, out, cfg);
            }
            for (n,ps,b) in methods {
                out.push_str("\n"); out.push_str(&" ".repeat(ind+2));
                out.push_str(n); out.push('(');
                for (i,p) in ps.iter().enumerate(){ if i>0 { out.push_str(", "); } out.push_str(p); }
                out.push_str(") -> "); fmt_expr(b, ind+2, out, cfg);
            }
            out.push_str("\n"); out.push_str(&" ".repeat(ind)); out.push('}');
        }
        New{name, overrides} => { out.push_str("new "); out.push_str(name); if !overrides.is_empty(){ out.push_str(" {"); for (i,(k,v)) in overrides.iter().enumerate(){ if i>0 { out.push_str(", "); } out.push_str(k); out.push_str(": "); fmt_expr(v, ind, out, cfg); } out.push('}'); } }
        ImportMod{id, alias} => { out.push_str("import "); out.push_str(id); if let Some(a)=alias { out.push_str(" as "); out.push_str(a); } }
        ImportFrom{id, items} => { out.push_str("from "); out.push_str(id); out.push_str(": "); for (i,(n,a)) in items.iter().enumerate(){ if i>0 { out.push_str(", "); } out.push_str(n); if let Some(alias)=a { out.push_str(" as "); out.push_str(alias); } } }
        Export(names) => { out.push_str("export: "); for (i,n) in names.iter().enumerate(){ if i>0 { out.push_str(", "); } out.push_str(n); } }
        Trait{name, methods} => { out.push_str("trait "); out.push_str(name); out.push_str(" {"); for m in methods { out.push_str(" "); out.push_str(m); out.push('('); out.push(')'); out.push(';'); } out.push_str(" }"); }
        Impl{tname,cname} => { out.push_str("impl "); out.push_str(tname); out.push_str(" for "); out.push_str(cname); }
        Match{scrut,arms} => { out.push_str("match "); fmt_expr(scrut, ind, out, cfg); out.push_str(" {"); for (pat,body) in arms { out.push_str(" "); match pat { crate::runtime::PatCase::Wild => out.push_str("_"), crate::runtime::PatCase::Class{name,bind} => { out.push_str(name); if let Some(b) = bind { out.push_str(" as "); out.push_str(b); } } } ; out.push_str(" -> "); fmt_expr(body, ind, out, cfg); out.push(';'); } out.push_str(" }"); }
        Prog(xs) => { for (i, s) in xs.iter().enumerate() { if i>0 { out.push('\n'); } out.push_str(&indent(ind)); fmt_expr(s, ind, out, cfg); } }
    }
}
