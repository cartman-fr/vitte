#[derive(Debug, Clone)]
pub enum Expr {
    Num(f64), Str(String), Var(String),
    List(Vec<Expr>), Tuple(Vec<Expr>), Record(Vec<(String,Expr)>),
    Unary{ op: char, a: Box<Expr>},
    Bin{ op: char, a: Box<Expr>, b: Box<Expr>},
    Call{ callee: Box<Expr>, args: Vec<Expr>},
    If{ c: Box<Expr>, a: Box<Expr>, b: Box<Expr>},
    Lambda{ params: Vec<String>, body: Box<Expr> },
    Assign{ name: String, e: Box<Expr> },
    FnDef{ name: String, params: Vec<String>, body: Box<Expr>},
    Import(String),
    Prog(Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Num(f64), Str(String), Id(String),
    Sym(char), Arrow, Comma, Colon, Eq, LParen, RParen, LBrack, RBrack, LBrace, RBrace,
    Semi,
    KwIf, KwThen, KwElse, KwFn, KwImport,
    EOF,
}

pub fn tokenize(src: &str) -> Vec<Token> {
    let s = src.as_bytes(); let mut i: usize = 0; let mut out = vec![];
    while let Some(&c) = s.get(i) {
        if c.is_ascii_whitespace() { i+=1; continue; }
        if c == b'#' { while let Some(&d)=s.get(i){ i+=1; if d==b'\n' { break; } } continue; }
        match c {
            b'0'..=b'9' => { let mut b=String::new(); b.push(c as char); i+=1;
                while let Some(&d)=s.get(i){ let ch=d as char; if ch.is_ascii_digit()||ch=='_'||ch=='.' { b.push(ch); i+=1; } else { break; } }
                out.push(Token::Num(b.replace('_',"").parse().unwrap())); }
            b'"' => { i+=1; let mut b=String::new();
                while let Some(&d)=s.get(i){ i+=1; if d==b'\\' { if let Some(&e)=s.get(i){ b.push(e as char); i+=1; } }
                    else if d==b'"' { break; } else { b.push(d as char) } }
                out.push(Token::Str(b)); }
            b'-' => { if s.get(i+1)==Some(&b'>'){ i+=2; out.push(Token::Arrow) } else { i+=1; out.push(Token::Sym('-')) } }
            b'+'|b'*'|b'/'|b'{'|b'}'|b'(' | b')' | b'[' | b']' | b',' | b':' | b'=' => {
                i+=1; out.push(match c {
                    b'+' => Token::Sym('+'), b'*' => Token::Sym('*'), b'/' => Token::Sym('/'),
                    b'{' => Token::LBrace, b'}' => Token::RBrace,
                    b'(' => Token::LParen, b')' => Token::RParen,
                    b'[' => Token::LBrack, b']' => Token::RBrack,
                    b',' => Token::Comma, b':' => Token::Colon, b'=' => Token::Eq, _ => unreachable!(),
                });
            }
            b';' | b'\n' => { i+=1; out.push(Token::Semi); }
            _ => {
                if (c as char).is_ascii_alphabetic() || c==b'_' {
                    let mut b = String::new(); b.push(c as char); i+=1;
                    while let Some(&d)=s.get(i){ let ch=d as char; if ch.is_ascii_alphanumeric()||ch=='_' { b.push(ch); i+=1; } else { break; } }
                    out.push(match b.as_str() {
                        "if"=>Token::KwIf, "then"=>Token::KwThen, "else"=>Token::KwElse,
                        "fn"=>Token::KwFn, "import"=>Token::KwImport,
                        _=>Token::Id(b)
                    });
                } else { i+=1; }
            }
        }
    }
    out.push(Token::EOF);
    out
}

// Pratt + defs/import
pub struct Parser{ toks: Vec<Token>, i: usize }
impl Parser{
    pub fn new(toks: Vec<Token>) -> Self { Self{ toks, i:0 } }
    fn peek(&self) -> &Token { self.toks.get(self.i).unwrap_or(&Token::EOF) }
    fn bump(&mut self) -> Token { let t = self.peek().clone(); self.i+=1; t }

    pub fn parse_program(&mut self) -> Result<Expr, String> {
        let mut xs = vec![];
        while !matches!(self.peek(), Token::EOF) {
            if matches!(self.peek(), Token::Semi) { self.i+=1; continue; }
            xs.push(self.parse_stmt()?);
            if matches!(self.peek(), Token::Semi) { self.i+=1; }
        }
        Ok(Expr::Prog(xs))
    }

    fn parse_stmt(&mut self) -> Result<Expr,String> {
        match self.peek() {
            Token::KwImport => {
                self.bump();
                let path = match self.bump() { Token::Str(s)=>s, _ => return Err("import: chaîne attendue".into()) };
                Ok(Expr::Import(path))
            }
            Token::KwFn => {
                self.bump();
                let name = match self.bump(){ Token::Id(s)=>s, _=>return Err("fn: ident attendu".into()) };
                if !matches!(self.bump(), Token::LParen) { return Err("'(' attendu".into()); }
                let mut params=vec![];
                if !matches!(self.peek(), Token::RParen) {
                    loop {
                        match self.bump(){ Token::Id(p)=>params.push(p), _=>return Err("param attendu".into()) }
                        if matches!(self.peek(), Token::Comma) { self.bump(); continue; }
                        break;
                    }
                }
                if !matches!(self.bump(), Token::RParen) { return Err("')' manquante".into()); }
                if !matches!(self.bump(), Token::Eq) { return Err("'=' attendu".into()); }
                let body = self.parse_expr_bp(0)?;
                Ok(Expr::FnDef{ name, params, body: Box::new(body) })
            }
            _ => {
                if let Token::Id(name) = self.peek().clone() {
                    if matches!(self.toks.get(self.i+1), Some(Token::Eq)) {
                        self.i+=2; let e = self.parse_expr_bp(0)?;
                        return Ok(Expr::Assign{ name, e: Box::new(e) });
                    }
                }
                self.parse_expr_bp(0)
            }
        }
    }

    fn lbp(tok: &Token) -> u8 {
        match tok {
            Token::LParen => 90,
            Token::Sym('*') | Token::Sym('/') => 70,
            Token::Sym('+') | Token::Sym('-') => 60,
            _ => 0,
        }
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr,String> {
        let mut lhs = self.parse_prefix()?;
        loop {
            if matches!(self.peek(), Token::LParen) {
                if 90 < min_bp { break; }
                self.bump();
                let mut args=vec![];
                if !matches!(self.peek(), Token::RParen){
                    loop{ args.push(self.parse_expr_bp(0)?); if matches!(self.peek(), Token::Comma){ self.bump(); continue; } break; }
                }
                if !matches!(self.bump(), Token::RParen){ return Err("')' manquante".into()); }
                lhs = Expr::Call{ callee: Box::new(lhs), args };
                continue;
            }
            let op = match self.peek(){ Token::Sym('+')=>'+', Token::Sym('-')=>'-', Token::Sym('*')=>'*', Token::Sym('/')=>'/', _=>break };
            let lbp = Self::lbp(self.peek()); if lbp < min_bp { break; }
            self.bump();
            let rhs = self.parse_expr_bp(lbp)?;
            lhs = Expr::Bin{ op, a: Box::new(lhs), b: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr,String> {
        use Token as T;
        match self.bump() {
            T::Num(n)=>Ok(Expr::Num(n)),
            T::Str(s)=>Ok(Expr::Str(s)),
            T::Id(x)=>Ok(Expr::Var(x)),
            T::Sym('-')=>{ let e=self.parse_expr_bp(80)?; Ok(Expr::Unary{ op:'-', a: Box::new(e) }) }
            T::LParen=>{
                // lambda? (x,y)->e
                let save = self.i;
                let mut params = vec![]; let mut ok = true;
                if !matches!(self.peek(), T::RParen) {
                    loop {
                        match self.bump(){ T::Id(p)=>params.push(p), _=>{ ok=false; break; } }
                        if matches!(self.peek(), T::Comma){ self.bump(); continue; }
                        break;
                    }
                }
                if ok && matches!(self.peek(), T::RParen) {
                    self.bump();
                    if matches!(self.peek(), T::Arrow) { self.bump(); let body = self.parse_expr_bp(0)?; return Ok(Expr::Lambda{ params, body: Box::new(body) }); }
                }
                self.i = save;
                let first = self.parse_expr_bp(0)?;
                if matches!(self.peek(), T::Comma){
                    let mut xs=vec![first]; while matches!(self.peek(), T::Comma){ self.bump(); xs.push(self.parse_expr_bp(0)?); }
                    if !matches!(self.bump(), T::RParen){ return Err("')' manquante".into()); }
                    Ok(Expr::Tuple(xs))
                } else {
                    if !matches!(self.bump(), T::RParen){ return Err("')' manquante".into()); }
                    Ok(first)
                }
            }
            T::LBrack=>{
                let mut xs=vec![];
                if !matches!(self.peek(), T::RBrack){
                    loop{ xs.push(self.parse_expr_bp(0)?); if matches!(self.peek(), T::Comma){ self.bump(); continue; } break; }
                }
                if !matches!(self.bump(), T::RBrack){ return Err("']' manquante".into()); }
                Ok(Expr::List(xs))
            }
            T::LBrace => {
                let mut fs = vec![];
                if !matches!(self.peek(), T::RBrace){
                    loop {
                        let k = match self.bump(){ T::Id(s)=>s, _=>return Err("clé attendue".into()) };
                        if !matches!(self.bump(), T::Colon){ return Err("':' manquant".into()); }
                        let v = self.parse_expr_bp(0)?;
                        fs.push((k, v));
                        if matches!(self.peek(), T::Comma){ self.bump(); continue; }
                        break;
                    }
                }
                if !matches!(self.bump(), T::RBrace){ return Err("'}' manquante".into()); }
                Ok(Expr::Record(fs))
            }
            T::KwIf => {
                let c = self.parse_expr_bp(0)?;
                if !matches!(self.bump(), T::KwThen) { return Err("'then' attendu".into()); }
                let a = self.parse_expr_bp(0)?;
                if !matches!(self.bump(), T::KwElse) { return Err("'else' attendu".into()); }
                let b = self.parse_expr_bp(0)?;
                Ok(Expr::If{ c:Box::new(c), a:Box::new(a), b:Box::new(b) })
            }
            t => Err(format!("préfixe inattendu: {:?}", t)),
        }
    }
}
