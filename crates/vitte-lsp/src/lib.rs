use vitte_ast::{tokenize, Parser};

pub fn serve_stdio(){
    use std::io::{self, Read, Write};
    let mut buf = String::new();
    loop {
        buf.clear();
        if io::stdin().read_line(&mut buf).unwrap_or(0)==0 { break; }
        if buf.trim().is_empty(){ continue; }
        if buf.contains("shutdown"){ println!("{"jsonrpc":"2.0","result":null,"id":1}"); break; }
        let toks = tokenize(&buf);
        let mut p = Parser::new(toks);
        let parsed = p.parse_program().is_ok();
        let reply = format!("{{"jsonrpc":"2.0","result":{{"ok":{}}},"id":1}}", parsed);
        let _ = writeln!(io::stdout(), "{}", reply);
    }
}
