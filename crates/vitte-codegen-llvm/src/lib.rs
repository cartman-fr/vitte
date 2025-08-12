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
    let mut strings: HashMap<String,String> = HashMap::new();
    fn fresh(r:&mut usize)->String{ *r+=1; format!("%{}", *r) }
    fn global_str_name(strings:&mut HashMap<String,String>, s:&str)->String{
        if let Some(n)=strings.get(s){ return n.clone(); }
        let name = format!("@.str{}", strings.len());
        strings.insert(s.to_string(), name.clone());
        name
    }
    fn emit_str_global(out:&mut String, name:&str, s:&str){
        let bytes = s.bytes().map(|b| format!("\{:02X}", b)).collect::<Vec<_>>().join("");
        let len = s.len()+1;
        out.push_str(&format!("{} = private unnamed_addr constant [{} x i8] c"{}\00"\n", name, len, s.replace("\\","\\5C").replace(""","\\22")));
    }
    fn emit_expr(out:&mut String, strings:&mut HashMap<String,String>, r:&mut usize, e:&Expr)->String{
        match e{
            Expr::Num(n)=>format!("{}", *n as i64),
            Expr::Str(s)=>{
                let g = global_str_name(strings, s);
                // We'll ensure globals are dumped after function via placeholder comment (not ideal, but ok for demo)
                format!("{} ; use {}", 0, g)
            }
            Expr::Bin{op:'+', a, b} => {
                let ra=emit_expr(out,strings,r,a); let rb=emit_expr(out,strings,r,b);
                let rc=fresh(r); out.push_str(&format!("  {} = add i64 {}, {}\n", rc, ra, rb)); rc
            }
            Expr::Bin{op:'-', a, b} => {
                let ra=emit_expr(out,strings,r,a); let rb=emit_expr(out,strings,r,b);
                let rc=fresh(r); out.push_str(&format!("  {} = sub i64 {}, {}\n", rc, ra, rb)); rc
            }
            Expr::Bin{op:'*', a, b} => {
                let ra=emit_expr(out,strings,r,a); let rb=emit_expr(out,strings,r,b);
                let rc=fresh(r); out.push_str(&format!("  {} = mul i64 {}, {}\n", rc, ra, rb)); rc
            }
            Expr::Bin{op:'/', a, b} => {
                let ra=emit_expr(out,strings,r,a); let rb=emit_expr(out,strings,r,b);
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
                                        let g = format!("@.str{}", strings.len());
                                        strings.insert(s.clone(), g.clone());
                                        out.push_str(&format!("  %p{} = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.fmt_s, i64 0, i64 0), i8* getelementptr inbounds ([{} x i8], [{} x i8]* {}, i64 0, i64 0))\n", strings.len(), s.len()+1, s.len()+1, g));
                                    }
                                    _ => {
                                        let rv=emit_expr(&mut out, &mut strings, &mut reg, &args[0]);
                                        let r2=fresh(&mut reg);
                                        out.push_str(&format!("  {} = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.fmt_i, i64 0, i64 0), i64 {})\n", r2, rv));
                                    }
                                }
                            }
                        }
                    }
                    Expr::If{c,a,b} => {
                        // evaluate cond to i64, branch if eq 0
                        let rc = emit_expr(&mut out, &mut strings, &mut reg, c);
                        let id = strings.len();
                        out.push_str(&format!("  %cmp{} = icmp ne i64 {}, 0\n", id, rc));
                        out.push_str(&format!("  br i1 %cmp{}, label %then{}, label %else{}\n", id, id, id));
                        out.push_str(&format!("then{}:\n", id));
                        let _ = emit_expr(&mut out, &mut strings, &mut reg, a);
                        out.push_str(&format!("  br label %end{}\n", id));
                        out.push_str(&format!("else{}:\n", id));
                        let _ = emit_expr(&mut out, &mut strings, &mut reg, b);
                        out.push_str(&format!("  br label %end{}\n", id));
                        out.push_str(&format!("end{}:\n", id));
                    }
                    _ => { let _=emit_expr(&mut out, &mut strings, &mut reg, x); }
                }
            }
        }
        _=> { let _=emit_expr(&mut out,&mut strings,&mut reg,e); }
    }
    out.push_str("  ret i32 0\n}\n");
    // dump string globals
    for (s,name) in strings.iter(){
        let len = s.len()+1;
        out.push_str(&format!("{} = private unnamed_addr constant [{} x i8] c"{}\00"\n", name, len, s.replace("\\","\\5C").replace(""","\\22")));
    }
    out
}
