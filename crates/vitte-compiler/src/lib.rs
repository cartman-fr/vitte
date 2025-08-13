//! vitte-compiler/src/lib.rs — Frontend minimal “prêt à rouler”
//!
//! Contenu dans 1 seul fichier pour accélérer :
//!  - Diagnostics (erreurs structurées)
//!  - Lexer (tokens + positions)
//!  - AST (expressions / statements)
//!  - Parser (descente récursive, précé­dences)
//!  - Codegen → vitte-core::bytecode::Chunk (LoadConst/Add/Sub/.../Print/Return)
//!
//! Langage MVP géré :
//!   - Statements: `print(expr);` | `expr;` | `return;`
//!   - Expressions: + - * / (gauche-associatif), parenthèses, nombres, chaînes, booléens, identifiants
//!
//! API publique : compile_str / compile_file / compile_path
//!
//! ⚠️ Ce n’est qu’un MVP : pas de variables locales, ni if/for, etc.
//!    L’objectif est de valider la chaîne source → bytecode → VM/désassembleur.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use thiserror::Error;

use vitte_core::bytecode::{chunk::ChunkFlags, Chunk, ConstPool, ConstValue, Op};

/// --------- API PUBLIQUE ---------

/// Compile du code source (chaîne) en Chunk bytecode.
pub fn compile_str(source: &str, main_file: Option<&str>) -> Result<Chunk> {
    let mut diags = Diagnostics::default();
    let mut lexer = Lexer::new(source, main_file.unwrap_or("<memory>"));
    let tokens = lexer.lex_all(&mut diags);
    bail_if_errors(&diags)?;

    let mut parser = Parser::new(tokens, lexer.file_name.clone());
    let program = parser.parse_program(&mut diags);
    bail_if_errors(&diags)?;

    let mut cg = Codegen::new(main_file);
    let chunk = cg.emit(&program)?;
    Ok(chunk)
}

/// Compile un fichier `.vit` en Chunk.
pub fn compile_file(path: impl AsRef<Path>) -> Result<Chunk> {
    let path = path.as_ref();
    let src = fs::read_to_string(path)
        .with_context(|| format!("Impossible de lire {}", path.display()))?;
    compile_str(&src, Some(path.to_string_lossy().as_ref()))
}

/// Compile à partir d’un chemin générique (pratique pour CLI).
pub fn compile_path(path: &Path) -> Result<Chunk> {
    compile_file(path)
}

/// --------- DIAGNOSTICS ---------

#[derive(Debug, Default)]
struct Diagnostics {
    errors: Vec<Diag>,
}

#[derive(Debug)]
struct Diag {
    file: String,
    line: usize,
    col: usize,
    msg: String,
}

fn bail_if_errors(diags: &Diagnostics) -> Result<()> {
    if diags.errors.is_empty() {
        return Ok(());
    }
    let mut s = String::new();
    for e in &diags.errors {
        use std::fmt::Write;
        let _ = writeln!(&mut s, "{}:{}:{}: {}", e.file, e.line, e.col, e.msg);
    }
    Err(anyhow!("Erreurs de compilation:\n{s}"))
}

impl Diagnostics {
    fn err(&mut self, file: &str, line: usize, col: usize, msg: impl Into<String>) {
        self.errors.push(Diag {
            file: file.to_string(),
            line,
            col,
            msg: msg.into(),
        });
    }
}

/// --------- LEXER ---------

#[derive(Debug, Clone, PartialEq)]
enum TokKind {
    // Mots-clés
    KwPrint,
    KwReturn,
    KwTrue,
    KwFalse,

    // Littéraux
    Number(f64),
    String(String),
    Ident(String),

    // Symboles
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Semicolon,

    // Fin
    Eof,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokKind,
    line: usize,
    col: usize,
}

struct Lexer<'a> {
    src: &'a str,
    file_name: String,
    it: std::str::CharIndices<'a>,
    peeked: Option<(usize, char)>,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str, file_name: &str) -> Self {
        Self {
            src,
            file_name: file_name.to_string(),
            it: src.char_indices(),
            peeked: None,
            line: 1,
            col: 1,
        }
    }

    fn lex_all(&mut self, diags: &mut Diagnostics) -> Vec<Token> {
        let mut out = Vec::new();
        loop {
            match self.next_token(diags) {
                Ok(t) => {
                    let eof = matches!(t.kind, TokKind::Eof);
                    out.push(t);
                    if eof { break; }
                }
                Err((line, col, msg)) => {
                    diags.err(&self.file_name, line, col, msg);
                    break;
                }
            }
        }
        out
    }

    fn next_token(&mut self, _diags: &mut Diagnostics) -> std::result::Result<Token, (usize, usize, String)> {
        self.skip_ws_and_comments();
        let (line, col) = (self.line, self.col);

        let c = match self.bump() {
            Some(c) => c,
            None => {
                return Ok(Token { kind: TokKind::Eof, line, col });
            }
        };

        let kind = match c {
            '(' => TokKind::LParen,
            ')' => TokKind::RParen,
            ';' => TokKind::Semicolon,
            '+' => TokKind::Plus,
            '-' => TokKind::Minus,
            '*' => TokKind::Star,
            '/' => TokKind::Slash,
            '"' => {
                let s = self.read_string()?;
                TokKind::String(s)
            }
            ch if ch.is_ascii_digit() => {
                let num = self.read_number(ch)?;
                TokKind::Number(num)
            }
            ch if is_ident_start(ch) => {
                let ident = self.read_ident(ch);
                match ident.as_str() {
                    "print" => TokKind::KwPrint,
                    "return" => TokKind::KwReturn,
                    "true" => TokKind::KwTrue,
                    "false" => TokKind::KwFalse,
                    _ => TokKind::Ident(ident),
                }
            }
            _ => return Err((line, col, format!("Caractère inattendu: {c:?}"))),
        };

        Ok(Token { kind, line, col })
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            let p = self.peek();
            match p {
                Some((_, ch)) if ch.is_ascii_whitespace() => {
                    self.bump();
                    continue;
                }
                Some((i, '/')) => {
                    // commentaire //…
                    if let Some((_, '/')) = self.peek2() {
                        // consommer jusqu'à fin de ligne
                        self.bump(); // '/'
                        self.bump(); // '/'
                        while let Some((_, ch)) = self.peek() {
                            if ch == '\n' { break; }
                            self.bump();
                        }
                        continue;
                    }
                }
                _ => break,
            }
        }
    }

    fn read_string(&mut self) -> std::result::Result<String, (usize, usize, String)> {
        let (mut line, mut col) = (self.line, self.col);
        let mut s = String::new();
        loop {
            match self.bump() {
                Some('"') => break,
                Some('\\') => {
                    match self.bump() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some(c) => {
                            return Err((line, col, format!("Échappement inconnu: \\{c}")));
                        }
                        None => return Err((line, col, "Fin de fichier dans la chaîne".into())),
                    }
                }
                Some(c) => s.push(c),
                None => return Err((line, col, "Fin de fichier dans la chaîne".into())),
            }
            line = self.line;
            col = self.col;
        }
        Ok(s)
    }

    fn read_number(&mut self, first: char) -> std::result::Result<f64, (usize, usize, String)> {
        let mut buf = String::new();
        buf.push(first);
        while let Some((_, ch)) = self.peek() {
            if ch.is_ascii_digit() || ch == '.' {
                self.bump();
                buf.push(ch);
            } else {
                break;
            }
        }
        buf.parse::<f64>()
            .map_err(|_| (self.line, self.col, format!("Nombre invalide: {buf}")))
    }

    fn read_ident(&mut self, first: char) -> String {
        let mut s = String::new();
        s.push(first);
        while let Some((_, ch)) = self.peek() {
            if is_ident_continue(ch) {
                self.bump();
                s.push(ch);
            } else {
                break;
            }
        }
        s
    }

    fn bump(&mut self) -> Option<char> {
        let (i, ch) = if let Some((i, ch)) = self.peeked.take().or_else(|| self.it.next()) {
            (i, ch)
        } else {
            return None;
        };

        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn peek(&mut self) -> Option<(usize, char)> {
        if self.peeked.is_none() {
            self.peeked = self.it.next();
        }
        self.peeked
    }

    fn peek2(&mut self) -> Option<(usize, char)> {
        // Regarde 2 chars en avant de manière simple
        let saved = self.peeked;
        let next = self.it.clone().next();
        self.peeked = saved;
        next
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' 
}
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' 
}

/// --------- AST ---------

#[derive(Debug)]
struct Program {
    stmts: Vec<Stmt>,
}

#[derive(Debug)]
enum Stmt {
    Print(Expr),
    Expr(Expr),
    Return,
}

#[derive(Debug)]
enum Expr {
    Number(f64),
    String(String),
    Bool(bool),
    Ident(String),
    Unary { op: UOp, rhs: Box<Expr> },
    Binary { op: BOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Group(Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
enum UOp { Neg }

#[derive(Debug, Clone, Copy)]
enum BOp { Add, Sub, Mul, Div }

/// --------- PARSER ---------

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    file: String,
}

impl Parser {
    fn new(tokens: Vec<Token>, file: String) -> Self {
        Self { tokens, pos: 0, file }
    }

    fn parse_program(&mut self, diags: &mut Diagnostics) -> Program {
        let mut stmts = Vec::new();
        while !self.check(TokKind::Eof) {
            match self.parse_stmt(diags) {
                Some(s) => stmts.push(s),
                None => {
                    // erreur : on tente de resynchroniser au prochain ';'
                    self.sync_to_semicolon();
                    self.advance(); // évite boucle infinie
                }
            }
        }
        Program { stmts }
    }

    fn parse_stmt(&mut self, diags: &mut Diagnostics) -> Option<Stmt> {
        if self.matches(&[TokKind::KwPrint]) {
            let expr = self.parse_expr(diags)?;
            self.consume_semicolon(diags)?;
            return Some(Stmt::Print(expr));
        }
        if self.matches(&[TokKind::KwReturn]) {
            self.consume_semicolon(diags)?;
            return Some(Stmt::Return);
        }
        // par défaut: expression-statement
        let expr = self.parse_expr(diags)?;
        self.consume_semicolon(diags)?;
        Some(Stmt::Expr(expr))
    }

    fn parse_expr(&mut self, diags: &mut Diagnostics) -> Option<Expr> {
        self.parse_add(diags)
    }

    fn parse_add(&mut self, diags: &mut Diagnostics) -> Option<Expr> {
        let mut expr = self.parse_mul(diags)?;
        loop {
            if self.matches(&[TokKind::Plus]) {
                let rhs = self.parse_mul(diags)?;
                expr = Expr::Binary { op: BOp::Add, lhs: Box::new(expr), rhs: Box::new(rhs) };
            } else if self.matches(&[TokKind::Minus]) {
                let rhs = self.parse_mul(diags)?;
                expr = Expr::Binary { op: BOp::Sub, lhs: Box::new(expr), rhs: Box::new(rhs) };
            } else {
                break;
            }
        }
        Some(expr)
    }

    fn parse_mul(&mut self, diags: &mut Diagnostics) -> Option<Expr> {
        let mut expr = self.parse_unary(diags)?;
        loop {
            if self.matches(&[TokKind::Star]) {
                let rhs = self.parse_unary(diags)?;
                expr = Expr::Binary { op: BOp::Mul, lhs: Box::new(expr), rhs: Box::new(rhs) };
            } else if self.matches(&[TokKind::Slash]) {
                let rhs = self.parse_unary(diags)?;
                expr = Expr::Binary { op: BOp::Div, lhs: Box::new(expr), rhs: Box::new(rhs) };
            } else {
                break;
            }
        }
        Some(expr)
    }

    fn parse_unary(&mut self, diags: &mut Diagnostics) -> Option<Expr> {
        if self.matches(&[TokKind::Minus]) {
            let rhs = self.parse_unary(diags)?;
            return Some(Expr::Unary { op: UOp::Neg, rhs: Box::new(rhs) });
        }
        self.parse_primary(diags)
    }

    fn parse_primary(&mut self, diags: &mut Diagnostics) -> Option<Expr> {
        if let Some(tok) = self.advance() {
            return match &tok.kind {
                TokKind::Number(n) => Some(Expr::Number(*n)),
                TokKind::String(s) => Some(Expr::String(s.clone())),
                TokKind::KwTrue => Some(Expr::Bool(true)),
                TokKind::KwFalse => Some(Expr::Bool(false)),
                TokKind::Ident(s) => Some(Expr::Ident(s.clone())),
                TokKind::LParen => {
                    let e = self.parse_expr(diags)?;
                    self.expect(TokKind::RParen, diags, tok.line, tok.col, "Parenthèse ')' attendue")?;
                    Some(Expr::Group(Box::new(e)))
                }
                _ => {
                    diags.err(&self.file, tok.line, tok.col, "Expression attendue");
                    None
                }
            };
        }
        None
    }

    fn matches(&mut self, kinds: &[TokKind]) -> bool {
        for k in kinds {
            if self.check(k.clone()) {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn check(&self, kind: TokKind) -> bool {
        if let Some(t) = self.peek() {
            std::mem::discriminant(&t.kind) == std::mem::discriminant(&kind)
        } else {
            false
        }
    }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.peek()?;
        self.pos += 1;
        Some(t)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn expect(&mut self, want: TokKind, diags: &mut Diagnostics, line: usize, col: usize, msg: &str) -> Option<()> {
        if self.check(want) {
            self.pos += 1;
            Some(())
        } else {
            diags.err(&self.file, line, col, msg);
            None
        }
    }

    fn consume_semicolon(&mut self, diags: &mut Diagnostics) -> Option<()> {
        if self.check(TokKind::Semicolon) {
            self.pos += 1;
            Some(())
        } else {
            let (l, c) = self.peek().map(|t| (t.line, t.col)).unwrap_or((0, 0));
            diags.err(&self.file, l, c, "Point-virgule ';' attendu");
            None
        }
    }

    fn sync_to_semicolon(&mut self) {
        while let Some(t) = self.peek() {
            if matches!(t.kind, TokKind::Semicolon | TokKind::Eof) {
                break;
            }
            self.pos += 1;
        }
    }
}

/// --------- CODEGEN ---------

struct Codegen {
    chunk: Chunk,
}

impl Codegen {
    fn new(main_file: Option<&str>) -> Self {
        let mut chunk = Chunk::new(ChunkFlags { stripped: false });
        if let Some(f) = main_file {
            chunk.debug.main_file = Some(f.to_string());
        }
        Self { chunk }
    }

    fn emit(mut self, program: &Program) -> Result<Chunk> {
        for stmt in &program.stmts {
            self.emit_stmt(stmt)?;
        }
        // S’assurer qu’on termine proprement
        self.chunk.ops.push(Op::Return);
        Ok(self.chunk)
    }

    fn emit_stmt(&mut self, s: &Stmt) -> Result<()> {
        match s {
            Stmt::Print(e) => {
                self.emit_expr(e)?;
                self.chunk.ops.push(Op::Print);
            }
            Stmt::Expr(e) => {
                self.emit_expr(e)?;
                self.chunk.ops.push(Op::Pop); // on ne garde pas la valeur
            }
            Stmt::Return => {
                self.chunk.ops.push(Op::Return);
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, e: &Expr) -> Result<()> {
        match e {
            Expr::Number(n) => {
                let ix = self.chunk.add_const(ConstValue::F64(*n));
                self.chunk.ops.push(Op::LoadConst(ix));
            }
            Expr::String(s) => {
                let ix = self.chunk.add_const(ConstValue::Str(s.clone()));
                self.chunk.ops.push(Op::LoadConst(ix));
            }
            Expr::Bool(b) => {
                self.chunk
                    .ops
                    .push(if *b { Op::LoadTrue } else { Op::LoadFalse });
            }
            Expr::Ident(name) => {
                // MVP: pas de variables -> on injecte le nom comme string pour visualisation
                let ix = self.chunk.add_const(ConstValue::Str(format!("<ident:{}>", name)));
                self.chunk.ops.push(Op::LoadConst(ix));
            }
            Expr::Unary { op: UOp::Neg, rhs } => {
                self.emit_expr(rhs)?;
                self.chunk.ops.push(Op::Neg);
            }
            Expr::Binary { op, lhs, rhs } => {
                self.emit_expr(lhs)?;
                self.emit_expr(rhs)?;
                match op {
                    BOp::Add => self.chunk.ops.push(Op::Add),
                    BOp::Sub => self.chunk.ops.push(Op::Sub),
                    BOp::Mul => self.chunk.ops.push(Op::Mul),
                    BOp::Div => self.chunk.ops.push(Op::Div),
                }
            }
            Expr::Group(inner) => {
                self.emit_expr(inner)?;
            }
        }
        Ok(())
    }
}

