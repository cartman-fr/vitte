use std::collections::HashMap;
use vitte_ast::Expr;

pub fn emit_ir(e:&Expr, filename:&str)->String{
    let mut out=String::new();
    out.push_str(&format!("; ModuleID = 'main'\nsource_filename = \"{}\"\n", filename));
    out.push_str("declare i32 @printf(i8*, ...)\n");
    out.push_str("@.fmt_i = private unnamed_addr constant [5 x i8] c"%ld\0A\00"\n");
    out.push_str("@.fmt_s = private unnamed_addr constant [4 x i8] c"%s\0A\00"\n\n");
    out.push_str("define i32 @main() {\n");
    let mut reg=0usize;
    fn fresh(r:&mut usize)->String{ *r+=1; format!("%{}", *r) }
    fn emit_expr(out:&mut String, r:&mut usize, e:&Expr)->String{
        match e{
            Expr::Num(n)=>format!("{}", *n as i64),
            Expr::Bin{op:'+', a, b} => {
                let ra=emit_expr(out,r,a); let rb=emit_expr(out,r,b);
                let rc=fresh(r); out.push_str(&format!("  {} = add i64 {}, {}\n", rc, ra, rb)); rc
            }
            Expr::Bin{op:'-', a, b} => {
                let ra=emit_expr(out,r,a); let rb=emit_expr(out,r,b);
                let rc=fresh(r); out.push_str(&format!("  {} = sub i64 {}, {}\n", rc, ra, rb)); rc
            }
            Expr::Bin{op:'*', a, b} => {
                let ra=emit_expr(out,r,a); let rb=emit_expr(out,r,b);
                let rc=fresh(r); out.push_str(&format!("  {} = mul i64 {}, {}\n", rc, ra, rb)); rc
            }
            Expr::Bin{op:'/', a, b} => {
                let ra=emit_expr(out,r,a); let rb=emit_expr(out,r,b);
                let rc=fresh(r); out.push_str(&format!("  {} = sdiv i64 {}, {}\n", rc, ra, rb)); rc
            }
            _=> "0".into(),
        }
    }
    match e{
        Expr::Prog(xs)=>{
            for x in xs {
                match x {
                    Expr::Call{callee, args} => {
                        if let Expr::Var(name) = &**callee {
                            if name=="print" && args.len()==1 {
                                match &args[0] {
                                    Expr::Str(s) => {
                                        out.push_str(&format!("  %p = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.fmt_s, i64 0, i64 0), i8* getelementptr inbounds ([{} x i8], [{} x i8]* @.str0, i64 0, i64 0))\n", s.len()+1, s.len()+1));
                                        out.push_str(&format!("@.str0 = private unnamed_addr constant [{} x i8] c"{}\00"\n", s.len()+1, s.replace("\\", "\\5C").replace(""","\\22")));
                                    }
                                    _ => {
                                        let rv=emit_expr(&mut out, &mut reg, &args[0]);
                                        let r2=fresh(&mut reg);
                                        out.push_str(&format!("  {} = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.fmt_i, i64 0, i64 0), i64 {})\n", r2, rv));
                                    }
                                }
                            }
                        }
                    }
                    _ => { let _=emit_expr(&mut out, &mut reg, x); }
                }
            }
        }
        _=> { let _=emit_expr(&mut out,&mut reg,e); }
    }
    out.push_str("  ret i32 0\n}\n");
    out
}
