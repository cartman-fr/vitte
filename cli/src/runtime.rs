
use std::collections::{HashMap, HashSet};
use color_eyre::eyre::{Result, eyre};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum Val {
    Num(f64),
    Str(String),
    Bool(bool),
    List(Vec<Val>),
    Record(HashMap<String, Val>),
    Class(ClassDef),
    Instance(ClassDef, HashMap<String, Val>),
    Lam(Vec<String>, Box<Expr>, Env),
    Builtin(std::rc::Rc<dyn Fn(Vec<Val>)->Result<Val>>),
    Unit,
}
impl Val {
    pub fn truthy(&self) -> bool {
        match self {
            Val::Bool(b) => *b,
            Val::Num(n) => *n != 0.0,
            Val::Str(s) => !s.is_empty(),
            Val::List(v) => !v.is_empty(),
            Val::Record(m) => !m.is_empty(),
            Val::Class(_) | Val::Instance(..) | Val::Lam(..) | Val::Builtin(_) => true,
            Val::Unit => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum LValue {
    Var(String),
    Index{ base: String, idx: Box<Expr> },
    Field{ base: String, name: String },
}
#[derive(Debug, Clone)]
pub enum Pattern {
    PVar(String),
    PTuple(Vec<Pattern>),
}
#[derive(Debug, Clone)]
pub enum PatCase {
    Wild,
    Class { name: String, bind: Option<String> },
}
    PVar(String),
    PTuple(Vec<Pattern>),
}

#[derive(Debug, Clone)]
pub struct Method { pub name: String, pub params: Vec<String>, pub body: Box<Expr> }

#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: String,
    pub parent: Option<Box<ClassDef>>,
    pub fields: HashMap<String, Expr>, // default expressions for instances
    pub methods: HashMap<String, Method>,
    pub static_fields: HashMap<String, Val>, // evaluated once at class def
    pub static_methods: HashMap<String, Method>,
}

#[derive(Debug, Clone)]
pub struct TraitDef { pub name: String, pub methods: std::collections::HashSet<String> }

pub enum Expr {
    Num(f64), Str(String), Bool(bool), Var(String),
    List(Vec<Expr>), Tuple(Vec<Expr>), Record(Vec<(String,Expr)>),
    Unary{op:String, e: Box<Expr>},
    Bin{op:String, a: Box<Expr>, b: Box<Expr>},
    Call{ callee: Box<Expr>, args: Vec<Expr> },
    Index{ target: Box<Expr>, index: Box<Expr> },
    Field{ target: Box<Expr>, name: String },
    If{ c: Box<Expr>, a: Box<Expr>, b: Box<Expr> },
    While{ c: Box<Expr>, body: Box<Expr> },
    For{ var: String, iter: Box<Expr>, body: Box<Expr> },
    ForKV{ k: String, v: String, iter: Box<Expr>, body: Box<Expr> },
    Lam{ params: Vec<String>, body: Box<Expr> },
    Assign{ name: String, e: Box<Expr> },
    AssignLv{ lv: LValue, e: Box<Expr> },
    AssignPat{ pat: Pattern, e: Box<Expr> },
    ClassDef{ name: String, parent: Option<String>, fields: Vec<(String,Expr)>, methods: Vec<(String, Vec<String>, Expr)>, sfields: Vec<(String,Expr)>, smethods: Vec<(String, Vec<String>, Expr)> },
    New{ name: String, overrides: Vec<(String,Expr)> },
    ImportMod{ id: String, alias: Option<String> },
    ImportFrom{ id: String, items: Vec<(String, Option<String>)> },
    Export(Vec<String>),
    Trait { name: String, methods: Vec<String> },
    Impl { tname: String, cname: String },
    Match { scrut: Box<Expr>, arms: Vec<(PatCase, Expr)> },
    Prog(Vec<Expr>),
}
pub type Env = HashMap<String, Val>;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Num(f64), Str(String), Id(String),
    Sym(char),
    Op(String),
    Semi,
    Kw(&'static str), // if then else while do true false for in import as from class new static pub export self
    Arrow,
    EOF,
}

pub fn tokenize(src: &str) -> Vec<Token> {
    let mut t = Tokenizer::new(src);
    let mut out = vec![];
    while let Some(tok) = t.next_token() { out.push(tok) }
    out
}
struct Tokenizer<'a>{ s:&'a [u8], i:usize }
impl<'a> Tokenizer<'a> {
    fn new(s:&'a str)->Self{ Self{ s:s.as_bytes(), i:0 } }
    fn peek(&self)->Option<u8>{ self.s.get(self.i).cloned() }
    fn bump(&mut self)->Option<u8>{ let c=self.peek()?; self.i+=1; Some(c) }
    fn ws(&mut self){
        while let Some(c)=self.peek() {
            if c.is_ascii_whitespace(){ self.i+=1; continue; }
            if c==b'#'{ while let Some(d)=self.peek(){ self.i+=1; if d==b'\n'{break} } continue; }
            break;
        }
    }
    fn next_token(&mut self)->Option<Token>{
        self.ws(); let c=self.bump()?;
        match c {
            b'-' => { if let Some(b'>') = self.peek().map(|x| x as char) { self.i += 1; return Some(Token::Arrow); } else { return Some(Token::Op("-".into())); } }
            b'!' => { if let Some(b'=') = self.peek() { self.i+=1; return Some(Token::Op("!=".into())); } else { return Some(Token::Op("!".into())); } }
            b'=' => { if let Some(b'=') = self.peek() { self.i+=1; return Some(Token::Op("==".into())); } else { return Some(Token::Sym('=')); } }
            b'<' => { if let Some(b'=') = self.peek() { self.i+=1; return Some(Token::Op("<=".into())); } else { return Some(Token::Op("<".into())); } }
            b'>' => { if let Some(b'=') = self.peek() { self.i+=1; return Some(Token::Op(">=".into())); } else { return Some(Token::Op(">".into())); } }
            b'&' => { if let Some(b'&') = self.peek() { self.i+=1; return Some(Token::Op("&&".into())); } else { return None; } }
            b'|' => { if let Some(b'|') = self.peek() { self.i+=1; return Some(Token::Op("||".into())); } else { return None; } }
            b'+'|b'*'|b'/' => return Some(Token::Op((c as char).to_string())),
            b'(' | b')' | b',' | b'[' | b']' | b'{' | b'}' | b':' | b'.' => return Some(Token::Sym(c as char)),
            b';'|b'\n' => return Some(Token::Semi),
            b'0'..=b'9'=>{
                let mut s=String::new(); s.push(c as char);
                while let Some(d)=self.peek(){
                    if (d as char).is_ascii_digit()||d==b'.'||d==b'_'{ s.push(d as char); self.i+=1; } else { break; }
                } return Some(Token::Num(s.replace('_',"").parse().unwrap()));
            }
            b'"'=>{
                let mut s=String::new();
                while let Some(d)=self.bump(){
                    if d==b'\\'{ if let Some(e)=self.bump(){ s.push(e as char) } }
                    else if d==b'"'{ break; }
                    else { s.push(d as char) }
                } return Some(Token::Str(s));
            }
            b'a'..=b'z'|b'A'..=b'Z'|b'_' => {
                let mut s=String::new(); s.push(c as char);
                while let Some(d)=self.peek(){
                    let ch=d as char;
                    if ch.is_ascii_alphanumeric()||ch=='_'{ s.push(ch); self.i+=1; } else { break; }
                }
                return Some(match s.as_str(){
                    "if"|"then"|"else"|"while"|"do"|"true"|"false"|"for"|"in"|"import"|"as"|"from"|"class"|"new"|"static"|"pub"|"export"|"self"|"trait"|"impl"|"match"
                        => Token::Kw(Box::leak(s.into_boxed_str())),
                    _=>Token::Id(s)
                });
            }
            _ => {}
        }
        None
    }
}

pub struct Parser{ toks:Vec<Token>, i:usize }
impl Parser {
    pub fn new(toks:Vec<Token>)->Self{ Self{ toks, i:0 } }
    fn peek(&self)->&Token{ self.toks.get(self.i).unwrap_or(&Token::EOF) }
    fn bump(&mut self) { self.i+=1; }
    fn eat(&mut self, want:&Token)->Result<()> { if self.peek()==want { self.i+=1; Ok(()) } else { Err(eyre!("attendu {:?}, trouvé {:?}", want, self.peek())) } }

    pub fn parse_program(&mut self)->Result<Expr>{
        let mut xs=vec![];
        while !matches!(self.peek(), Token::EOF){
            if matches!(self.peek(), Token::Semi){ self.i+=1; continue; }
            xs.push(self.parse_stmt()?);
            if matches!(self.peek(), Token::Semi){ self.i+=1; }
        }
        Ok(Expr::Prog(xs))
    }
    fn parse_stmt(&mut self)->Result<Expr>{
        // trait Name { method(a,b); ... }  (only names required)
        if matches!(self.peek(), Token::Kw("trait")) {
            self.bump();
            let name = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("nom de trait attendu")) };
            if !matches!(self.peek(), Token::Sym('{')) { return Err(eyre!("'{' attendu après nom de trait")); }
            self.bump();
            let mut methods = vec![];
            while !matches!(self.peek(), Token::Sym('}')) {
                let m = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("nom de méthode attendu")) };
                if matches!(self.peek(), Token::Sym('(')) {
                    // skip params list
                    self.bump();
                    while !matches!(self.peek(), Token::Sym(')')) {
                        match self.peek() {
                            Token::Id(_) => { self.bump(); }
                            Token::Sym(',') => { self.bump(); }
                            _ => return Err(eyre!("paramètre ou ')' attendu")),
                        }
                    }
                    self.eat(&Token::Sym(')'))?;
                }
                methods.push(m);
                if matches!(self.peek(), Token::Semi) { self.bump(); }
            }
            self.eat(&Token::Sym('}'))?;
            return Ok(Expr::Trait { name, methods });
        }
        // impl Trait for Class
        if matches!(self.peek(), Token::Kw("impl")) {
            self.bump();
            let tname = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("nom de trait attendu")) };
            if !matches!(self.peek(), Token::Kw("for")) { return Err(eyre!("attendu 'for'")); }
            self.bump();
            let cname = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("nom de classe attendu")) };
            return Ok(Expr::Impl { tname, cname });
        }
        // export: a, b, C
        if matches!(self.peek(), Token::Kw("export")) {
            self.bump();
            if !matches!(self.peek(), Token::Sym(':')) { return Err(eyre!("attendu ':' après 'export'")); }
            self.bump();
            let mut items = vec![];
            loop {
                match self.peek().clone() {
                    Token::Id(s) => { self.bump(); items.push(s); }
                    _ => return Err(eyre!("identifiant attendu dans export")),
                }
                if matches!(self.peek(), Token::Sym(',')) { self.bump(); continue; }
                break;
            }
            return Ok(Expr::Export(items));
        }

        // class Name [: Parent] { members }
        if matches!(self.peek(), Token::Kw("class")) {
            self.bump();
            let name = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("nom de classe attendu")) };
            let mut parent: Option<String> = None;
            if matches!(self.peek(), Token::Sym(':')) {
                self.bump();
                parent = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); Some(s) }, _=>return Err(eyre!("nom de parent attendu après ':'")) };
            }
            if !matches!(self.peek(), Token::Sym('{')) { return Err(eyre!("'{' attendu après nom de classe")); }
            self.bump();
            let mut fields: Vec<(String,Expr)> = vec![];
            let mut methods: Vec<(String, Vec<String>, Expr)> = vec![];
            let mut sfields: Vec<(String,Expr)> = vec![];
            let mut smethods: Vec<(String, Vec<String>, Expr)> = vec![];
            while !matches!(self.peek(), Token::Sym('}')) {
                let is_static = matches!(self.peek(), Token::Kw("static"));
                if is_static { self.bump(); }
                let id = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("identifiant attendu dans class")) };
                if matches!(self.peek(), Token::Sym(':')) {
                    self.bump();
                    let e = self.parse_expr()?;
                    if is_static { sfields.push((id, e)); } else { fields.push((id, e)); }
                } else if matches!(self.peek(), Token::Sym('(')) {
                    self.bump();
                    let mut params: Vec<String> = vec![];
                    if !matches!(self.peek(), Token::Sym(')')) {
                        loop {
                            match self.peek().clone() {
                                Token::Id(p) => { self.bump(); params.push(p); }
                                _ => return Err(eyre!("paramètre attendu")),
                            }
                            if matches!(self.peek(), Token::Sym(',')) { self.bump(); continue; }
                            break;
                        }
                    }
                    self.eat(&Token::Sym(')'))?;
                    if !matches!(self.peek(), Token::Arrow) { return Err(eyre!("'->' attendu dans méthode")); }
                    self.bump();
                    let body = self.parse_expr()?;
                    if is_static { smethods.push((id, params, body)); } else { methods.push((id, params, body)); }
                } else {
                    return Err(eyre!("attendu ':' ou '(' après identifiant dans class"));
                }
                if matches!(self.peek(), Token::Semi) { self.bump(); }
            }
            self.eat(&Token::Sym('}'))?;
            return Ok(Expr::ClassDef{ name, parent, fields, methods, sfields, smethods });
        }

        // destructuring
        let save = self.i;
        if let Some(pat) = self.try_parse_pattern() {
            if matches!(self.peek(), Token::Sym('=')) {
                self.bump();
                let e=self.parse_expr()?;
                return Ok(Expr::AssignPat{ pat, e: Box::new(e) });
            } else { self.i = save; }
        } else { self.i = save; }

        // lvalue assignment
        let save2 = self.i;
        if let Some(lv) = self.try_parse_lvalue() {
            if matches!(self.peek(), Token::Sym('=')) {
                self.bump();
                let e = self.parse_expr()?;
                return Ok(Expr::AssignLv{ lv, e: Box::new(e) });
            } else { self.i = save2; }
        } else { self.i = save2; }

        // imports
        if matches!(self.peek(), Token::Kw("from")) {
            self.bump();
            let id = match self.peek().clone() { Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("module attendu après 'from'")) };
            if !matches!(self.peek(), Token::Sym(':')) { return Err(eyre!("attendu ':' après le nom du module")); }
            self.bump();
            let mut items: Vec<(String, Option<String>)> = vec![];
            loop {
                let name = match self.peek().clone() { Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("identifiant attendu")) };
                let mut alias = None;
                if matches!(self.peek(), Token::Kw("as")) { self.bump(); alias = match self.peek().clone() { Token::Id(s)=>{ self.bump(); Some(s) }, _=>return Err(eyre!("alias attendu")) }; }
                items.push((name, alias));
                if matches!(self.peek(), Token::Sym(',')) { self.bump(); continue; }
                break;
            }
            return Ok(Expr::ImportFrom{ id, items });
        }
        if matches!(self.peek(), Token::Kw("import")) {
            self.bump();
            let id = match self.peek().clone() { Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("module attendu")) };
            let alias = if matches!(self.peek(), Token::Kw("as")) {
                self.bump();
                match self.peek().clone() { Token::Id(s)=>{ self.bump(); Some(s) }, _=>return Err(eyre!("alias attendu")) }
            } else { None };
            return Ok(Expr::ImportMod{ id, alias });
        }

        // default
        self.parse_expr()
    }

    fn try_parse_pattern(&mut self) -> Option<Pattern> {
        match self.peek().clone() {
            Token::Id(s) => { self.bump(); Some(Pattern::PVar(s)) }
            Token::Sym('(') => {
                let save = self.i; self.bump();
                let mut items = vec![];
                if !matches!(self.peek(), Token::Sym(')')) {
                    loop {
                        match self.peek().clone() {
                            Token::Id(s) => { self.bump(); items.push(Pattern::PVar(s)); }
                            _ => { self.i = save; return None; }
                        }
                        if matches!(self.peek(), Token::Sym(',')) { self.bump(); continue; }
                        break;
                    }
                }
                if self.eat(&Token::Sym(')')).is_err() { self.i = save; return None; }
                Some(Pattern::PTuple(items))
            }
            _ => None,
        }
    }
    fn try_parse_lvalue(&mut self) -> Option<LValue> {
        let save = self.i;
        let name = match self.peek().clone() {
            Token::Id(s) => { self.bump(); s }
            _ => return None,
        };
        match self.peek() {
            Token::Sym('[') => { self.bump(); let idx = self.parse_expr().ok()?; if self.eat(&Token::Sym(']')).is_err(){ self.i = save; return None; } Some(LValue::Index{ base: name, idx: Box::new(idx) }) }
            Token::Sym('.') => { self.bump(); if let Token::Id(field)=self.peek().clone(){ self.bump(); Some(LValue::Field{ base: name, name: field }) } else { self.i=save; None } }
            _ => Some(LValue::Var(name)),
        }
    }

    pub fn parse_expr(&mut self)->Result<Expr>{
        match self.peek() {
            Token::Kw("if") => {
                self.bump(); let c=self.parse_expr()?;
                if !matches!(self.peek(), Token::Kw("then")) { return Err(eyre!("attendu 'then'")); }
                self.bump(); let a=self.parse_expr()?;
                if !matches!(self.peek(), Token::Kw("else")) { return Err(eyre!("attendu 'else'")); }
                self.bump(); let b=self.parse_expr()?;
                return Ok(Expr::If{ c:Box::new(c), a:Box::new(a), b:Box::new(b) });
            }
            Token::Kw("while") => {
                self.bump();
                let c=self.parse_expr()?;
                if !matches!(self.peek(), Token::Kw("do")) { return Err(eyre!("attendu 'do'")); }
                self.bump();
                let body=self.parse_expr()?;
                return Ok(Expr::While{ c:Box::new(c), body:Box::new(body) });
            }
            Token::Kw("for") => {
                self.bump();
                // maybe (k,v)
                if matches!(self.peek(), Token::Sym('(')) {
                    self.bump();
                    let k = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("identifiant attendu")) };
                    if !matches!(self.peek(), Token::Sym(',')) { return Err(eyre!("',' attendu")); }
                    self.bump();
                    let v = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("identifiant attendu")) };
                    if !matches!(self.peek(), Token::Sym(')')) { return Err(eyre!("')' attendu")); }
                    self.bump();
                    if !matches!(self.peek(), Token::Kw("in")) { return Err(eyre!("attendu 'in'")); }
                    self.bump();
                    let iter=self.parse_expr()?;
                    if !matches!(self.peek(), Token::Kw("do")) { return Err(eyre!("attendu 'do'")); }
                    self.bump();
                    let body=self.parse_expr()?;
                    return Ok(Expr::ForKV{ k, v, iter:Box::new(iter), body:Box::new(body) });
                }
                let var = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("identifiant attendu après 'for'")) };
                if !matches!(self.peek(), Token::Kw("in")) { return Err(eyre!("attendu 'in'")); }
                self.bump();
                let iter=self.parse_expr()?;
                if !matches!(self.peek(), Token::Kw("do")) { return Err(eyre!("attendu 'do'")); }
                self.bump();
                let body=self.parse_expr()?;
                return Ok(Expr::For{ var, iter:Box::new(iter), body:Box::new(body) });
            }
            Token::Kw("new") => {
                self.bump();
                let name = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("nom de classe attendu après 'new'")) };
                let mut overrides: Vec<(String,Expr)> = vec![];
                if matches!(self.peek(), Token::Sym('{')) {
                    self.bump();
                    if !matches!(self.peek(), Token::Sym('}')) {
                        loop {
                            let k = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("clé d'override attendue")) };
                            self.eat(&Token::Sym(':'))?;
                            let v = self.parse_expr()?;
                            overrides.push((k, v));
                            if matches!(self.peek(), Token::Sym(',')) { self.bump(); continue; }
                            break;
                        }
                    }
                    self.eat(&Token::Sym('}'))?;
                }
                return Ok(Expr::New{ name, overrides });
            }
            _ => {}
        }
        if matches!(self.peek(), Token::Kw("match")) {
            self.bump();
            let scrut = self.parse_expr()?;
            if !matches!(self.peek(), Token::Sym('{')) { return Err(eyre!("'{' attendu après match")); }
            self.bump();
            let mut arms = vec![];
            while !matches!(self.peek(), Token::Sym('}')) {
                // pattern: Class [as x] | _
                let pat = if matches!(self.peek(), Token::Id("_".into())) { self.bump(); PatCase::Wild } else {
                    let cname = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("pattern attendu")) };
                    let mut bind=None; if matches!(self.peek(), Token::Kw("as")) { self.bump(); bind = match self.peek().clone(){ Token::Id(s)=>{ self.bump(); Some(s) }, _=>return Err(eyre!("identifiant après 'as'")) }; }
                    PatCase::Class{ name: cname, bind }
                };
                if !matches!(self.peek(), Token::Op("=>".into())) { /* allow ':' or '->' as separator fallback */ }
                if matches!(self.peek(), Token::Sym(':')) { self.bump(); }
                if matches!(self.peek(), Token::Arrow) { self.bump(); }
                let body = self.parse_expr()?;
                arms.push((pat, body));
                if matches!(self.peek(), Token::Sym(',')) || matches!(self.peek(), Token::Semi) { self.bump(); }
            }
            self.eat(&Token::Sym('}'))?;
            return Ok(Expr::Match{ scrut: Box::new(scrut), arms });
        }
        self.parse_bp(0)
    }

    fn lbp(op:&str)->u8 { match op { "||"=>1, "&&"=>2, "=="|"!="=>3, "<"|"<="|">"|">="=>4, "+"|"-"=>5, "*"|"/"=>6, _=>0 } }
    fn parse_bp(&mut self, min_bp:u8)->Result<Expr>{
        let mut lhs = self.parse_prefix()?;
        loop {
            let op = if let Token::Op(op) = self.peek() { op.clone() } else { break };
            let lbp = Self::lbp(&op); if lbp<min_bp { break; }
            self.bump();
            let mut rhs = self.parse_prefix()?;
            loop {
                let next = if let Token::Op(op2)=self.peek(){ op2.clone() } else { break };
                let rbp = Self::lbp(&next);
                if rbp>lbp { self.bump(); let rhs2=self.parse_bp(rbp)?; rhs = Expr::Bin{ op: next, a:Box::new(rhs), b:Box::new(rhs2) }; } else { break; }
            }
            lhs = Expr::Bin{ op, a:Box::new(lhs), b:Box::new(rhs) };
        }
        Ok(lhs)
    }
    fn parse_prefix(&mut self)->Result<Expr>{
        if let Token::Op(op) = self.peek().clone() {
            if op=="-" || op=="!" { self.bump(); let e=self.parse_prefix()?; return Ok(Expr::Unary{ op, e:Box::new(e) }); }
        }
        self.parse_postfix()
    }
    fn parse_postfix(&mut self)->Result<Expr>{
        let mut node = self.parse_atom()?;
        loop {
            match self.peek() {
                Token::Sym('(') => { self.bump(); let mut args=vec![]; if !matches!(self.peek(), Token::Sym(')')){ loop{ let e=self.parse_expr()?; args.push(e); if matches!(self.peek(), Token::Sym(',')){ self.bump(); continue; } break; } } self.eat(&Token::Sym(')'))?; node=Expr::Call{ callee:Box::new(node), args }; }
                Token::Sym('[') => { self.bump(); let idx=self.parse_expr()?; self.eat(&Token::Sym(']'))?; node=Expr::Index{ target:Box::new(node), index:Box::new(idx) }; }
                Token::Sym('.') => { self.bump(); let name=match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("identifiant attendu après '.'")) }; node=Expr::Field{ target:Box::new(node), name }; }
                _ => break
            }
        }
        Ok(node)
    }
    fn parse_atom(&mut self)->Result<Expr>{
        match self.peek().clone() {
            Token::Num(n)=>{ self.bump(); Ok(Expr::Num(n)) }
            Token::Str(s)=>{ self.bump(); Ok(Expr::Str(s)) }
            Token::Kw("true") => { self.bump(); Ok(Expr::Bool(true)) }
            Token::Kw("false") => { self.bump(); Ok(Expr::Bool(false)) }
            Token::Id(name)=>{ self.bump(); Ok(Expr::Var(name)) }
            Token::Sym('(')=>{
                // lambda?
                let save=self.i; self.bump();
                let mut params=vec![]; let mut ok=true;
                if !matches!(self.peek(), Token::Sym(')')){
                    loop {
                        match self.peek().clone(){ Token::Id(p)=>{ self.bump(); params.push(p); }, _=>{ ok=false; break; } }
                        if matches!(self.peek(), Token::Sym(',')){ self.bump(); continue; }
                        break;
                    }
                }
                if ok && matches!(self.peek(), Token::Sym(')')){
                    self.bump();
                    if matches!(self.peek(), Token::Arrow){
                        self.bump();
                        let body=self.parse_expr()?;
                        return Ok(Expr::Lam{ params, body:Box::new(body) });
                    }
                }
                // group or tuple
                self.i=save; self.bump();
                let first=self.parse_expr()?;
                if matches!(self.peek(), Token::Sym(',')){
                    let mut xs=vec![first];
                    while matches!(self.peek(), Token::Sym(',')){ self.bump(); xs.push(self.parse_expr()?); }
                    self.eat(&Token::Sym(')'))?; Ok(Expr::Tuple(xs))
                } else { self.eat(&Token::Sym(')'))?; Ok(first) }
            }
            Token::Sym('[')=>{ self.bump(); let mut xs=vec![]; if !matches!(self.peek(), Token::Sym(']')){ loop{ xs.push(self.parse_expr()?); if matches!(self.peek(), Token::Sym(',')){ self.bump(); continue; } break; } } self.eat(&Token::Sym(']'))?; Ok(Expr::List(xs)) }
            Token::Sym('{') => { self.bump(); let mut fs:Vec<(String,Expr)>=vec![]; if !matches!(self.peek(), Token::Sym('}')){ loop{ let k=match self.peek().clone(){ Token::Id(s)=>{ self.bump(); s }, _=>return Err(eyre!("clé de record attendue")) }; self.eat(&Token::Sym(':'))?; let v=self.parse_expr()?; fs.push((k,v)); if matches!(self.peek(), Token::Sym(',')){ self.bump(); continue; } break; } } self.eat(&Token::Sym('}'))?; Ok(Expr::Record(fs)) }
            _ => Err(eyre!("atome inattendu: {:?}", self.peek())),
        }
    }
}

// --- Rendering & utilities ---
pub fn render(v: Val)->String{
    match v{
        Val::Num(n)=>{ if (n - (n as i64 as f64)).abs()<1e-9 { format!("{}", n as i64) } else { format!("{}", n) } }
        Val::Str(s)=>s,
        Val::Bool(b)=> b.to_string(),
        Val::List(xs)=>{ let inner:Vec<String>=xs.into_iter().map(render).collect(); format!("[{}]", inner.join(", ")) }
        Val::Record(mut m)=>{
            let mut keys: Vec<_> = m.keys().cloned().collect(); keys.sort();
            let inner: Vec<String> = keys.into_iter().map(|k| format!("{}: {}", k, render(m.remove(&k).unwrap()))).collect();
            format!("{{{}}}", inner.join(", "))
        }
        Val::Class(c)=>format!("<class {}>", c.name),
        Val::Instance(c, _)=>format!("<{} instance>", c.name),
        Val::Lam(..)|Val::Builtin(_)=>"<fn>".into(),
        Val::Unit=>"()".into(),
    }
}

fn call_value(cal: Val, args: Vec<Val>) -> Result<Val> {
    match cal {
        Val::Builtin(f) => f(args),
        Val::Lam(params, body, mut cap) => {
            for (i,p) in params.iter().enumerate() {
                cap.insert(p.clone(), args.get(i).cloned().unwrap_or(Val::Unit));
            }
            eval_expr_with_env(&mut cap, &body, None)
        }
        _ => Err(eyre!("appel d'une valeur non-fonction")),
    }
}

fn equal(a:&Val, b:&Val)->bool{
    match (a,b){
        (Val::Num(x), Val::Num(y)) => (*x - *y).abs() < 1e-9,
        (Val::Str(x), Val::Str(y)) => x == y,
        (Val::Bool(x), Val::Bool(y)) => x==y,
        (Val::List(xs), Val::List(ys)) => xs.len()==ys.len() && xs.iter().zip(ys).all(|(x,y)| equal(x,y)),
        (Val::Record(mx), Val::Record(my)) => {
            if mx.len()!=my.len() { return false; }
            for (k, vx) in mx { if let Some(vy) = my.get(k) { if !equal(vx, vy) { return false; } } else { return false; } }
            true
        }
        (Val::Unit, Val::Unit) => true,
        _ => false,
    }
}

fn find_method(cls: &ClassDef, name: &str) -> Option<Method> {
    if let Some(m) = cls.methods.get(name) { return Some(m.clone()); }
    if let Some(p) = &cls.parent { return find_method(p, name); }
    None
}
fn class_is_or_derived(cls:&ClassDef, name:&str)->bool { if cls.name==name { true } else if let Some(p)=&cls.parent { class_is_or_derived(p, name) } else { false } }

fn find_static_method(cls: &ClassDef, name: &str) -> Option<Method> {
    if let Some(m) = cls.static_methods.get(name) { return Some(m.clone()); }
    if let Some(p) = &cls.parent { return find_static_method(p, name); }
    None
}
fn has_trait(cls:&ClassDef, t:&str)->bool{ if cls.traits.contains(t){ true } else if let Some(p)=&cls.parent { has_trait(p,t) } else { false } }

fn get_static_field<'a>(cls: &'a ClassDef, name: &str) -> Option<Val> {
    if let Some(v) = cls.static_fields.get(name) { return Some(v.clone()); }
    if let Some(p) = &cls.parent { return get_static_field(p, name); }
    None
}

fn bind_pattern(env: &mut Env, pat: &Pattern, v: Val) -> Result<()> {
    match pat {
        Pattern::PVar(name) => { env.insert(name.clone(), v); Ok(()) }
        Pattern::PTuple(ps) => {
            let xs = match v { Val::List(vs) => vs, _ => return Err(eyre!("déstructuration attend une liste/tuple")) };
            if xs.len() < ps.len() { return Err(eyre!("déstructuration: pas assez d'éléments")); }
            for (i,p) in ps.iter().enumerate() { bind_pattern(env, p, xs[i].clone())?; }
            Ok(())
        }
    }
}

fn prelude() -> (Env, HashSet<String>) {
    use std::rc::Rc;
    let mut env: Env = HashMap::new();
    let mut names: HashSet<String> = HashSet::new();
    macro_rules! add { ($n:expr, $v:expr) => {{ env.insert($n.into(), $v); names.insert($n.into()); }} }
    add!("print", Val::Builtin(Rc::new(|args|{ println!("{}", render(args.get(0).cloned().unwrap_or(Val::Unit))); Ok(Val::Unit) })));
    add!("len", Val::Builtin(Rc::new(|args|{ let n=match args.get(0){ Some(Val::List(xs))=>xs.len() as f64, Some(Val::Str(s))=>s.len() as f64, _=>0.0 }; Ok(Val::Num(n)) })));
    add!("push", Val::Builtin(Rc::new(|args|{ let mut xs=match args.get(0).cloned().unwrap_or(Val::List(vec![])){ Val::List(v)=>v, _=>vec![] }; if let Some(x)=args.get(1){ xs.push(x.clone()); } Ok(Val::List(xs)) })));
    add!("str", Val::Builtin(Rc::new(|args| Ok(Val::Str(render(args.get(0).cloned().unwrap_or(Val::Unit)))))));
    add!("map", Val::Builtin(Rc::new(|args|{ if args.len()<2 { return Err(eyre!("map(fn, liste)")); } let f=args[0].clone(); let xs=match &args[1]{ Val::List(v)=>v.clone(), _=>return Err(eyre!("map: 2e arg non-liste")) }; let mut out=Vec::with_capacity(xs.len()); for x in xs { out.push(call_value(f.clone(), vec![x])?); } Ok(Val::List(out)) })));
    add!("classof", Val::Builtin(Rc::new(|args|{
        Ok(match args.get(0){
            Some(Val::Instance(c,_))=>Val::Str(c.name.clone()),
            Some(Val::Class(c))=>Val::Str(c.name.clone()),
            _=>Val::Str("".into())
        })
    })));
    add!("implements", Val::Builtin(Rc::new(|args|{
        if args.len()<2 { return Err(eyre!("implements(obj, \"Trait\")")); }
        let trait_name = match &args[1]{ Val::Str(s)=>s.clone(), _=>return Err(eyre!("nom de trait attendu")) };
        Ok(Val::Bool(match &args[0]{
            Val::Instance(c,_) => has_trait(c, &trait_name),
            Val::Class(c) => has_trait(c, &trait_name),
            _=>false
        }))
    })));
    add!("each", Val::Builtin(Rc::new(|args|{ if args.len()<2 { return Err(eyre!("each(fn, liste)")); } let f=args[0].clone(); let xs=match &args[1]{ Val::List(v)=>v.clone(), _=>return Err(eyre!("each: 2e arg non-liste")) }; for x in xs { let _=call_value(f.clone(), vec![x])?; } Ok(Val::Unit) })));
    // super(name, self, arg1, arg2, ...)
    add!("super", Val::Builtin(Rc::new(|args|{
        if args.len()<2 { return Err(eyre!("super(name,self,...)")); }
        let name = match &args[0] { Val::Str(s)=>s.clone(), _=>return Err(eyre!("super: 1er arg string attendu")) };
        let selfv = args[1].clone();
        let rest = args[2..].to_vec();
        let (cls, fields) = match selfv { Val::Instance(c,f)=> (c,f), _=> return Err(eyre!("super: self doit être instance")) };
        let p = match cls.parent { Some(ref bx)=> bx.as_ref().clone(), None => return Err(eyre!("super: pas de parent")) };
        if let Some(m) = find_method(&p, &name) {
            let mut cap = HashMap::new();
            cap.insert("self".into(), Val::Instance((*cls).clone(), fields.clone())); // self = current instance
            return call_value(Val::Lam(m.params.into_iter().filter(|x| x!="self").collect(), m.body, cap), rest);
        }
        Err(eyre!("méthode parent non trouvée"))
    })));

    (env, names)
}

pub fn eval_expr_with_env(env:&mut Env, e:&Expr, cwd: Option<&Path>)->Result<Val>{
    match e {
        Expr::Num(n)=>Ok(Val::Num(*n)),
        Expr::Str(s)=>Ok(Val::Str(s.clone())),
        Expr::Bool(b)=>Ok(Val::Bool(*b)),
        Expr::Var(x)=>env.get(x).cloned().ok_or_else(|| eyre!(format!("variable non définie: {}", x))),
        Expr::List(xs)=>{ let mut v=vec![]; for x in xs { v.push(eval_expr_with_env(env,x,cwd)?); } Ok(Val::List(v)) }
        Expr::Tuple(xs)=>{ let mut v=vec![]; for x in xs { v.push(eval_expr_with_env(env,x,cwd)?); } Ok(Val::List(v)) }
        Expr::Record(fs)=>{ let mut m=HashMap::new(); for (k,v) in fs { m.insert(k.clone(), eval_expr_with_env(env,v,cwd)?); } Ok(Val::Record(m)) }
        Expr::Unary{op,e} => {
            let v = eval_expr_with_env(env, e, cwd)?;
            match (op.as_str(), v) {
                ("-", Val::Num(n)) => Ok(Val::Num(-n)),
                ("!", x) => Ok(Val::Bool(!x.truthy())),
                _ => Err(eyre!("unaire non supporté")),
            }
        }
        Expr::Bin{op,a,b}=>{
            if op=="&&" { let A=eval_expr_with_env(env,a,cwd)?; return Ok(Val::Bool(A.truthy() && eval_expr_with_env(env,b,cwd)?.truthy())); }
            if op=="||" { let A=eval_expr_with_env(env,a,cwd)?; return Ok(Val::Bool(A.truthy() || eval_expr_with_env(env,b,cwd)?.truthy())); }
            let A=eval_expr_with_env(env,a,cwd)?; let B=eval_expr_with_env(env,b,cwd)?;
            match (op.as_str(), A, B){
                ("+", Val::Num(x), Val::Num(y)) => Ok(Val::Num(x+y)),
                ("-", Val::Num(x), Val::Num(y)) => Ok(Val::Num(x-y)),
                ("*", Val::Num(x), Val::Num(y)) => Ok(Val::Num(x*y)),
                ("/", Val::Num(x), Val::Num(y)) => Ok(Val::Num(x/y)),
                ("==", x, y) => Ok(Val::Bool(equal(&x,&y))),
                ("!=", x, y) => Ok(Val::Bool(!equal(&x,&y))),
                ("<", Val::Num(x), Val::Num(y)) => Ok(Val::Bool(x<y)),
                ("<=", Val::Num(x), Val::Num(y)) => Ok(Val::Bool(x<=y)),
                (">", Val::Num(x), Val::Num(y)) => Ok(Val::Bool(x>y)),
                (">=", Val::Num(x), Val::Num(y)) => Ok(Val::Bool(x>=y)),
                _ => Err(eyre!("opération non supportée")),
            }
        }
        Expr::Call{callee,args}=>{
            let cal=eval_expr_with_env(env,callee,cwd)?;
            let vals:Vec<Val>=args.iter().map(|a|eval_expr_with_env(env,a,cwd)).collect::<Result<_>>()?;
            call_value(cal, vals)
        }
        Expr::Index{target, index} => {
            let t = eval_expr_with_env(env, target, cwd)?;
            let i = match eval_expr_with_env(env, index, cwd)? { Val::Num(n) => n as i64, _ => return Err(eyre!("index non numérique")) };
            match t {
                Val::List(xs) => { let k=if i<0 {(xs.len() as i64 + i) as usize } else { i as usize }; xs.get(k).cloned().ok_or_else(|| eyre!("index hors limites")) }
                Val::Str(s) => { let chars:Vec<char>=s.chars().collect(); let k=if i<0 {(chars.len() as i64 + i) as usize } else { i as usize }; chars.get(k).map(|c| Val::Str(c.to_string())).ok_or_else(|| eyre!("index hors limites")) }
                _ => Err(eyre!("indexation sur non-indexable")),
            }
        }
        Expr::Field{target, name} => {
            let t = eval_expr_with_env(env, target, cwd)?;
            match t {
                Val::Record(m) => m.get(name).cloned().ok_or_else(|| eyre!(format!("champ '{}' absent", name))),
                Val::Instance(cls, fields) => {
                    if name.starts_with('_') {
                        match env.get("__in_class") { Some(Val::Str(s)) if s==&cls.name => {}, _ => return Err(eyre!(format!("membre privé '{}'", name))) }
                    }
                    if let Some(v) = fields.get(name).cloned() { return Ok(v); }
                    if let Some(meth) = find_method(&cls, name) {
                        let mut cap = env.clone();
                        cap.insert("self".into(), Val::Instance(cls.clone(), fields.clone()));
                        cap.insert("__in_class".into(), Val::Str(cls.name.clone()));
                        return Ok(Val::Lam(meth.params.into_iter().filter(|p| p!="self").collect(), meth.body, cap));
                    }
                    Err(eyre!(format!("champ/méthode '{}' introuvable", name)))
                }
                Val::Class(cls) => {
                    if name.starts_with('_') {
                        match env.get("__in_class") { Some(Val::Str(s)) if s==&cls.name => {}, _ => return Err(eyre!(format!("membre de classe privé '{}'", name))) }
                    }
                    if let Some(v) = get_static_field(&cls, name) { return Ok(v); }
                    if let Some(m) = find_static_method(&cls, name) {
                        let mut cap = env.clone();
                        cap.insert("__in_class".into(), Val::Str(cls.name.clone()));
                        return Ok(Val::Lam(m.params, m.body, cap));
                    }
                    Err(eyre!(format!("membre de classe '{}' introuvable", name)))
                }
                _ => Err(eyre!("accès champ sur non-record/instance/classe")),
            }
        }
        Expr::If{c,a,b}=>{ let cv=eval_expr_with_env(env,c,cwd)?; if cv.truthy(){ eval_expr_with_env(env,a,cwd) } else { eval_expr_with_env(env,b,cwd) } }
        Expr::While{c,body}=>{
            let mut last = Val::Unit;
            loop {
                let cv = eval_expr_with_env(env, c, cwd)?;
                if !cv.truthy() { break; }
                last = eval_expr_with_env(env, body, cwd)?;
            }
            Ok(last)
        }
        Expr::For{var, iter, body} => {
            let xs = match eval_expr_with_env(env, iter, cwd)? { Val::List(v) => v, _ => return Err(eyre!("for ... in attend une liste")) };
            let mut last = Val::Unit;
            for v in xs {
                env.insert(var.clone(), v);
                last = eval_expr_with_env(env, body, cwd)?;
            }
            Ok(last)
        }
        Expr::ForKV{k, v, iter, body} => {
            let rec = match eval_expr_with_env(env, iter, cwd)? { Val::Record(m) => m, _ => return Err(eyre!("for (k,v) in attend un record")) };
            let mut keys: Vec<_> = rec.keys().cloned().collect(); keys.sort();
            let mut last = Val::Unit;
            for key in keys {
                let val = rec.get(&key).cloned().unwrap_or(Val::Unit);
                env.insert(k.clone(), Val::Str(key));
                env.insert(v.clone(), val);
                last = eval_expr_with_env(env, body, cwd)?;
            }
            Ok(last)
        }
        Expr::Lam{params, body}=>Ok(Val::Lam(params.clone(), body.clone(), env.clone())),
        Expr::Assign{name,e}=>{ let v=eval_expr_with_env(env,e,cwd)?; env.insert(name.clone(), v.clone()); Ok(v) }
        Expr::AssignLv{ lv, e } => {
            let vnew = eval_expr_with_env(env, e, cwd)?;
            match lv {
                LValue::Var(name) => { env.insert(name.clone(), vnew.clone()); Ok(vnew) }
                LValue::Index{ base, idx } => {
                    let mut basev = env.get(&base).cloned().ok_or_else(|| eyre!(format!("variable non définie: {}", base)))?;
                    let i = match eval_expr_with_env(env, idx, cwd)? { Val::Num(n)=> n as i64, _=>return Err(eyre!("index non numérique")) };
                    match &mut basev {
                        Val::List(xs) => {
                            let k = if i<0 { (xs.len() as i64 + i) as usize } else { i as usize };
                            if k>=xs.len() { return Err(eyre!("index hors limites")); }
                            xs[k] = vnew.clone();
                            env.insert(base.clone(), basev.clone());
                            Ok(vnew)
                        }
                        _ => Err(eyre!("assignation d'index sur non-liste")),
                    }
                }
                LValue::Field{ base, name } => {
                    let mut basev = env.get(&base).cloned().ok_or_else(|| eyre!(format!("variable non définie: {}", base)))?;
                    match &mut basev {
                        Val::Record(m) => {
                            m.insert(name.clone(), vnew.clone());
                            env.insert(base.clone(), basev.clone());
                            Ok(vnew)
                        }
                        Val::Instance(cls, mut fs) => {
                            fs.insert(name.clone(), vnew.clone());
                            env.insert(base.clone(), Val::Instance(cls, fs));
                            Ok(vnew)
                        }
                        Val::Class(mut cls) => {
                            cls.static_fields.insert(name.clone(), vnew.clone());
                            env.insert(base.clone(), Val::Class(cls));
                            Ok(vnew)
                        }
                        _ => Err(eyre!("assignation de champ sur non-record/instance/classe")),
                    }
                }
            }
        }
        Expr::AssignPat{ pat, e } => { let v=eval_expr_with_env(env,e,cwd)?; bind_pattern(env, pat, v)?; Ok(Val::Unit) }
        Expr::ClassDef{ name, parent, fields, methods, sfields, smethods } => {
            // Resolve parent (if any)
            let parent_cls = if let Some(pid) = parent {
                match env.get(pid).cloned() { Some(Val::Class(c)) => Some(Box::new(c)), _ => return Err(eyre!(format!("classe parent '{}' introuvable", pid))) }
            } else { None };
            // Evaluate static fields now
            let mut sf: HashMap<String, Val> = HashMap::new();
            for (k, dv) in sfields { sf.insert(k.clone(), eval_expr_with_env(env, dv, cwd)?); }
            let mut meths: HashMap<String, Method> = HashMap::new();
            for (n, ps, b) in methods { meths.insert(n.clone(), Method{ name:n.clone(), params: ps.clone(), body: Box::new(b.clone())}); }
            let mut smeths: HashMap<String, Method> = HashMap::new();
            for (n, ps, b) in smethods { smeths.insert(n.clone(), Method{ name:n.clone(), params: ps.clone(), body: Box::new(b.clone())}); }
            let mut f: HashMap<String, Expr> = HashMap::new();
            for (k,v) in fields { f.insert(k.clone(), v.clone()); }
            let cls = ClassDef{ name: name.clone(), parent: parent_cls, fields: f, methods: meths, static_fields: sf, static_methods: smeths, pub_fields: Default::default(), pub_methods: Default::default(), pub_static_fields: Default::default(), pub_static_methods: Default::default(), traits: Default::default() };
            env.insert(name.clone(), Val::Class(cls));
            Ok(Val::Unit)
        }
        Expr::New{ name, overrides } => {
            let cls = match env.get(name).cloned() { Some(Val::Class(c)) => c, _=> return Err(eyre!(format!("classe '{}' introuvable", name))) };
            // Build default fields from parent chain (parent first)
            fn gather_fields(cls:&ClassDef, acc:&mut Vec<(String,Expr)>) {
                if let Some(p) = &cls.parent { gather_fields(p, acc); }
                for (k,v) in &cls.fields { acc.push((k.clone(), v.clone())); }
            }
            let mut all: Vec<(String,Expr)> = vec![]; gather_fields(&cls, &mut all);
            let mut fs: HashMap<String, Val> = HashMap::new();
            for (k, dv) in all { fs.insert(k.clone(), eval_expr_with_env(env, &dv, cwd)?); }
            for (k, v) in overrides { fs.insert(k.clone(), eval_expr_with_env(env, &v, cwd)?); }
            let inst = Val::Instance(cls.clone(), fs);
            // Call init if exists (child method overrides parent; parent init call via super("init", self))
            if let Some(m) = find_method(&cls, "init") {
                let mut cap = env.clone();
                cap.insert("self".into(), inst.clone());
                let _ = call_value(Val::Lam(m.params.into_iter().filter(|p| p!="self").collect(), m.body, cap), vec![])?;
            }
            Ok(inst)
        }
        Expr::ImportMod{ id, alias } => {
            let (mut menv, builtins) = prelude();
            let path1 = cwd.map(|c| c.join(format!("{}.vitte", id))).unwrap_or_else(|| PathBuf::from(format!("{}.vitte", id)));
            let path2 = cwd.map(|c| c.join("modules").join(format!("{}.vitte", id))).unwrap_or_else(|| PathBuf::from(format!("modules/{}.vitte", id)));
            let path = if path1.exists() { path1 } else { path2 };
            let s = std::fs::read_to_string(&path).map_err(|e| eyre!("import `{}` introuvable: {}", id, e))?;
            let toks = tokenize(&s);
            let mut p = Parser::new(toks);
            let prog = p.parse_program()?;
            let _ = eval_expr_with_env(&mut menv, &prog, path.parent())?;
            // gather exports: if __exports__ present (Record with keys), restrict to those
            let export_keys: Option<HashSet<String>> = match menv.get("__exports__") {
                Some(Val::Record(m)) => Some(m.keys().cloned().collect()),
                _ => None,
            };
            let mut rec = HashMap::new();
            for (k,v) in menv.into_iter() {
                if builtins.contains(&k) { continue; }
                if let Some(ref allow) = export_keys { if !allow.contains(&k) { continue; } }
                rec.insert(k, v);
            }
            let name = alias.clone().unwrap_or_else(|| id.clone());
            env.insert(name, Val::Record(rec));
            Ok(Val::Unit)
        }
        Expr::ImportFrom{ id, items } => {
            let (mut menv, builtins) = prelude();
            let path1 = cwd.map(|c| c.join(format!("{}.vitte", id))).unwrap_or_else(|| PathBuf::from(format!("{}.vitte", id)));
            let path2 = cwd.map(|c| c.join("modules").join(format!("{}.vitte", id))).unwrap_or_else(|| PathBuf::from(format!("modules/{}.vitte", id)));
            let path = if path1.exists() { path1 } else { path2 };
            let s = std::fs::read_to_string(&path).map_err(|e| eyre!("import `{}` introuvable: {}", id, e))?;
            let toks = tokenize(&s);
            let mut p = Parser::new(toks);
            let prog = p.parse_program()?;
            let _ = eval_expr_with_env(&mut menv, &prog, path.parent())?;
            let export_keys: Option<HashSet<String>> = match menv.get("__exports__") {
                Some(Val::Record(m)) => Some(m.keys().cloned().collect()),
                _ => None,
            };
            for (n, alias) in items {
                if builtins.contains(n) { continue; }
                if let Some(ref allow) = export_keys { if !allow.contains(n) { return Err(eyre!(format!("'{}' n'est pas exporté par {}", n, id))); } }
                let val = menv.get(n).cloned().ok_or_else(|| eyre!(format!("export '{}' absent dans {}", n, id)))?;
                env.insert(alias.clone().unwrap_or_else(|| n.clone()), val);
            }
            Ok(Val::Unit)
        }
        Expr::Trait{ name, methods } => {
            let set: std::collections::HashSet<String> = methods.into_iter().cloned().collect();
            // encode trait as Record of method names -> true
            let mut m = HashMap::new(); for k in &set { m.insert(k.clone(), Val::Bool(true)); }
            env.insert(name.clone(), Val::Record(m));
            Ok(Val::Unit)
        }
        Expr::Impl{ tname, cname } => {
            // mark class as implementing trait
            let mut cls = match env.get(&cname).cloned(){ Some(Val::Class(c))=>c, _=>return Err(eyre!(format!("classe '{}' introuvable", cname))) };
            cls.traits.insert(tname.clone());
            env.insert(cname.clone(), Val::Class(cls));
            Ok(Val::Unit)
        }
        Expr::Match{ scrut, arms } => {
            let v = eval_expr_with_env(env, scrut, cwd)?;
            for (pat, body) in arms {
                match pat {
                    PatCase::Wild => { return eval_expr_with_env(env, body, cwd); }
                    PatCase::Class{ name, bind } => {
                        match &v {
                            Val::Instance(c,_) => {
                                // match on class name or ancestor
                                if class_is_or_derived(&c, &name) {
                                    if let Some(b) = bind { env.insert(b.clone(), v.clone()); }
                                    return eval_expr_with_env(env, body, cwd);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(Val::Unit)
        }
        Expr::Export(names) => {
            let mut m = HashMap::new();
            for n in names { m.insert(n.clone(), Val::Bool(true)); }
            env.insert("__exports__".into(), Val::Record(m));
            Ok(Val::Unit)
        }
        Expr::Prog(xs)=>{ let mut last=Val::Unit; for s in xs { last=eval_expr_with_env(env,s,cwd)?; } Ok(last) }
    }
}

/// Evaluate a program. When `capture` is true, `print` collects to a buffer instead of writing stdout.
pub fn eval_with_capture(prog:&Expr, capture: bool, cwd: Option<&Path>)->Result<String>{
    let out_lines = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let (mut env, _builtins) = prelude();
    if capture {
        let out = out_lines.clone();
        env.insert("print".into(), Val::Builtin(std::rc::Rc::new(move |args|{
            let s = render(args.get(0).cloned().unwrap_or(Val::Unit));
            out.borrow_mut().push(s);
            Ok(Val::Unit)
        })) );
    }
    let _ = eval_expr_with_env(&mut env, prog, cwd)?;
    Ok(out_lines.borrow().join("\n"))
}
pub fn eval(prog:&Expr)->Result<String>{ eval_with_capture(prog, false, None) }
