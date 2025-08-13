//! tokenizer.rs — Analyse lexicale (lexer) du langage Vitte (.vit)
//!
//! Objectifs :
//! - Zéro dépendance, Unicode-aware, lignes/colonnes/offsets précis.
//! - Commentaires : `// ...` et `/* ... */` **imbriqués**.
//! - Littéraux :
//!     - bool: `true`/`false` ; null: `null`
//!     - int: décimal/hex `0x..`/bin `0b..`, underscores autorisés
//!     - float: `1.`, `.5`, `1.0`, `1e+9`, underscores autorisés
//!     - str: `"..."` échappes `\n \t \r \\ \" \xHH \u{...}`
//!     - bytes: `b"..."` (mêmes échappes, émet bytes bruts)
//!     - hex-bytes: `hex"DE AD BE EF"` (espaces `_` tolérés)
//! - Opérateurs/punctuations larges : `== != <= >= && || -> => :: .. ... += -= *= /= %= &= |= ^= <<= >>= :=` etc.
//! - Identifiants Unicode (`XID_Start`/`XID_Continue` approximé via heuristique std).
//!
//! API :
//!   let mut lx = Lexer::new(src);
//!   while let tok = lx.next_token()? { if tok.kind == TokenKind::Eof {break;} ... }
//!   // ou: let toks = tokenize(src)?;
//!
//! NB: Les valeurs des littéraux sont **cuites** (ex: String/Vec<u8>/i64/f64) ET
//!     on conserve la lexème brute dans Token (champ `lexeme`) pour debug.

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, unused_must_use)]

use std::fmt;

/* ───────────────────────── Positions & spans ───────────────────────── */

/// Position 1-based (ligne/col) + offset 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
    pub offset: usize,
}
impl Pos {
    fn start() -> Self { Self { line: 1, col: 1, offset: 0 } }
}

/// Tranche source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Pos,
    pub end: Pos,
}
impl Span {
    pub fn merge(a: Span, b: Span) -> Span { Span { start: a.start, end: b.end } }
}

/* ───────────────────────── Erreurs lexing ───────────────────────── */

#[derive(Debug, Clone)]
pub struct LexError {
    pub span: Span,
    pub msg: String,
}
impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, "{} (line {}, col {})",
            self.msg, self.span.start.line, self.span.start.col
        )
    }
}
impl std::error::Error for LexError {}

/* ───────────────────────── Tokens ───────────────────────── */

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// Vue texte brute (slice de la source)
    pub lexeme: String,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Fin
    Eof,

    // Ident & mots-clés
    Ident(String),
    // Mots-clés (enum dédiée pour branchements rapides)
    KwLet, KwFn, KwIf, KwElse, KwWhile, KwFor, KwReturn,
    KwTrue, KwFalse, KwNull,
    KwMatch, KwBreak, KwContinue, KwStruct, KwEnum, KwImpl,
    KwUse, KwAs, KwFrom, KwIn, KwMut, KwConst, KwPub, KwMod, KwExtern,

    // Littéraux
    Int { value: i64 },
    Float { value: f64 },
    Str { value: String },
    Bytes { value: Vec<u8> },

    // Punctuation / opérateurs
    // Simples
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Dot, Semicolon, Colon, Question, At,
    // Affectation & combinaisons
    Assign, PlusAssign, MinusAssign, StarAssign, SlashAssign, PercentAssign,
    AndAssign, OrAssign, XorAssign, ShlAssign, ShrAssign,
    // Binaires
    Plus, Minus, Star, Slash, Percent,
    And, Or, Xor, Shl, Shr,
    // Comparaisons
    EqEq, Ne, Lt, Le, Gt, Ge,
    // Logiques
    AndAnd, OrOr, Not,
    // Extras
    Arrow, FatArrow, ColonColon, Range, RangeEq, Ellipsis, // -> => :: .. ..= ...
    Tilde, // si un jour utile

    // Spéciaux (rare)
    Hash, // '#'
}

/* ───────────────────────── Lexer ───────────────────────── */

pub struct Lexer<'a> {
    src: &'a str,
    chars: std::str::CharIndices<'a>,
    /// lookahead courant
    look: Option<(usize, char)>,
    /// position courante (début du *prochain* token)
    pos: Pos,
    /// dernier point lu (avancer correctement en cas de lookahead)
    last_offset: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut chars = src.char_indices();
        let look = chars.next();
        Self {
            src, chars, look,
            pos: Pos::start(),
            last_offset: 0,
        }
    }

    /// Tokenise intégralement la source.
    pub fn tokenize_all(mut self) -> Result<Vec<Token>, LexError> {
        let mut v = Vec::<Token>::new();
        loop {
            let t = self.next_token()?;
            let end = matches!(t.kind, TokenKind::Eof);
            v.push(t);
            if end { break; }
        }
        Ok(v)
    }

    /// Pointe le prochain token sans le consommer.
    pub fn peek_token(&mut self) -> Result<Token, LexError> {
        let save = self.clone();
        let t = self.next_token()?;
        *self = save;
        Ok(t)
    }

    /// Lit le prochain token (ignore espaces/commentaires).
    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_ws_and_comments()?;

        let start = self.pos;
        let (i, ch) = match self.look { Some(p) => p, None => return Ok(self.mk_token(start, start, TokenKind::Eof)) };

        // ident / mot-clé
        if is_ident_start(ch) {
            return self.lex_ident_or_keyword();
        }

        // nombres
        if ch.is_ascii_digit() || ch == '.' || ch == '+' || ch == '-' {
            if let Some(tok) = self.try_number()? {
                return Ok(tok);
            }
        }

        // strings / bytes / hex bytes
        if ch == '"' {
            return self.lex_string(false);
        }
        if ch == 'b' {
            if self.starts_with("b\"") {
                self.bump(); // 'b'
                return self.lex_bytes_literal();
            }
        }
        if ch == 'h' && self.starts_with("hex\"") {
            return self.lex_hex_bytes_literal();
        }

        // Shebang en tête de fichier (#!...)
        if i == 0 && self.starts_with("#!") {
            self.eat_line();
            return self.next_token();
        }

        // opérateurs/punctuations
        self.lex_punct_or_op()
    }

    /* ────── core ────── */

    fn mk_token(&self, start: Pos, end: Pos, kind: TokenKind) -> Token {
        let s = &self.src[start.offset .. end.offset];
        Token { kind, span: Span { start, end }, lexeme: s.to_string() }
    }

    fn bump(&mut self) -> Option<char> {
        let (i, ch) = self.look?;
        self.last_offset = i;
        // avancer pos
        if ch == '\n' {
            self.pos.line += 1; self.pos.col = 1;
        } else {
            self.pos.col += 1;
        }
        self.pos.offset = i + ch.len_utf8();

        self.look = self.chars.next();
        Some(ch)
    }

    fn peek(&self) -> Option<char> { self.look.map(|(_,c)| c) }

    fn peek2(&self) -> Option<(char, char)> {
        let mut c = self.chars.clone();
        let _ = self.look?;
        if let Some((_, a)) = self.look {
            if let Some((_, b)) = c.next() {
                return Some((a, b));
            }
        }
        None
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos.offset..].starts_with(s)
    }

    fn eat_line(&mut self) {
        while let Some((_, c)) = self.look {
            self.bump();
            if c == '\n' { break; }
        }
    }

    fn skip_ws_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            // espaces
            while matches!(self.peek(), Some(c) if c.is_whitespace()) { self.bump(); }
            // commentaires
            if self.starts_with("//") {
                self.eat_line();
                continue;
            }
            if self.starts_with("/*") {
                self.skip_block_comment()?;
                continue;
            }
            break;
        }
        Ok(())
    }

    fn skip_block_comment(&mut self) -> Result<(), LexError> {
        // commentaires imbriqués
        let start = self.pos;
        self.bump(); // '/'
        self.bump(); // '*'
        let mut depth = 1usize;
        while let Some(c) = self.bump() {
            if c == '/' && self.peek() == Some('*') {
                let _ = self.bump();
                depth += 1;
            } else if c == '*' && self.peek() == Some('/') {
                let _ = self.bump();
                depth -= 1;
                if depth == 0 { return Ok(()); }
            }
        }
        Err(self.err_at(start, "commentaire /* ... */ non terminé"))
    }

    fn err_here(&self, msg: &str) -> LexError { self.err_at(self.pos, msg) }
    fn err_at(&self, pos: Pos, msg: &str) -> LexError {
        LexError { span: Span { start: pos, end: self.pos }, msg: msg.to_string() }
    }

    /* ────── ident / keywords ────── */

    fn lex_ident_or_keyword(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        let mut s = String::new();
        // premier
        if let Some(c) = self.peek() { s.push(c); self.bump(); }
        // suite
        while let Some(c) = self.peek() {
            if is_ident_continue(c) { s.push(c); self.bump(); } else { break; }
        }
        let end = self.pos;

        // mots-clés
        let kind = match s.as_str() {
            "let" => TokenKind::KwLet,
            "fn" => TokenKind::KwFn,
            "if" => TokenKind::KwIf,
            "else" => TokenKind::KwElse,
            "while" => TokenKind::KwWhile,
            "for" => TokenKind::KwFor,
            "return" => TokenKind::KwReturn,
            "true" => TokenKind::KwTrue,
            "false" => TokenKind::KwFalse,
            "null" => TokenKind::KwNull,
            "match" => TokenKind::KwMatch,
            "break" => TokenKind::KwBreak,
            "continue" => TokenKind::KwContinue,
            "struct" => TokenKind::KwStruct,
            "enum" => TokenKind::KwEnum,
            "impl" => TokenKind::KwImpl,
            "use" => TokenKind::KwUse,
            "as" => TokenKind::KwAs,
            "from" => TokenKind::KwFrom,
            "in" => TokenKind::KwIn,
            "mut" => TokenKind::KwMut,
            "const" => TokenKind::KwConst,
            "pub" => TokenKind::KwPub,
            "mod" => TokenKind::KwMod,
            "extern" => TokenKind::KwExtern,
            _ => TokenKind::Ident(s),
        };
        Ok(self.mk_token(start, end, kind))
    }

    /* ────── nombres ────── */

    fn try_number(&mut self) -> Result<Option<Token>, LexError> {
        let save = self.clone();
        match self.lex_number() {
            Ok(tok) => Ok(Some(tok)),
            Err(_) => { *self = save; Ok(None) }
        }
    }

    fn lex_number(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        let mut raw = String::new();

        // signe optionnel
        if matches!(self.peek(), Some('+') | Some('-')) {
            raw.push(self.bump().unwrap());
        }

        // hex/bin prefix ?
        if self.starts_with("0x") || self.starts_with("0X") {
            raw.push_str("0x"); self.bump(); self.bump();
            let (digits, any) = self.collect_while(|c| c.is_ascii_hexdigit() || c == '_');
            if !any { return Err(self.err_at(start, "hex: chiffre attendu")); }
            raw.push_str(&digits);
            let val = i64_from_base(&raw, 16).ok_or_else(|| self.err_at(start, "entier hex invalide"))?;
            return Ok(self.mk_token(start, self.pos, TokenKind::Int { value: val }));
        }
        if self.starts_with("0b") || self.starts_with("0B") {
            raw.push_str("0b"); self.bump(); self.bump();
            let (digits, any) = self.collect_while(|c| c == '0' || c == '1' || c == '_');
            if !any { return Err(self.err_at(start, "binaire: chiffre attendu")); }
            raw.push_str(&digits);
            let val = i64_from_base(&raw, 2).ok_or_else(|| self.err_at(start, "entier binaire invalide"))?;
            return Ok(self.mk_token(start, self.pos, TokenKind::Int { value: val }));
        }

        // décimal/float
        let mut any_digit = false;
        // partie entière
        let (digits, had) = self.collect_while(|c| c.is_ascii_digit() || c == '_');
        if had { any_digit = true; raw.push_str(&digits); }

        // fraction .digits
        let mut is_float = false;
        if self.peek() == Some('.') {
            // lookahead: si pas ".." ni "..." ni ".ident", on considère fraction si suit digit
            let second = self.peek2().map(|(_, b)| b);
            if matches!(second, Some(d) if d.is_ascii_digit()) {
                is_float = true; raw.push('.'); self.bump(); // '.'
                let (fd, hadf) = self.collect_while(|c| c.is_ascii_digit() || c == '_');
                if !hadf { return Err(self.err_at(start, "chiffres attendus après '.'")); }
                raw.push_str(&fd);
            }
        }

        // exposant
        if matches!(self.peek(), Some('e'|'E')) {
            is_float = true; raw.push(self.bump().unwrap());
            if matches!(self.peek(), Some('+'|'-')) { raw.push(self.bump().unwrap()); }
            let (ed, had) = self.collect_while(|c| c.is_ascii_digit() || c == '_');
            if !had { return Err(self.err_at(start, "exposant: chiffre attendu")); }
            raw.push_str(&ed);
        }

        if !any_digit && !is_float {
            return Err(self.err_at(start, "nombre attendu"));
        }

        // parse
        let cooked: String = raw.chars().filter(|&c| c != '_').collect();
        if is_float || cooked.contains('.') || cooked.contains('e') || cooked.contains('E') {
            let x: f64 = cooked.parse().map_err(|_| self.err_at(start, "flottant invalide"))?;
            Ok(self.mk_token(start, self.pos, TokenKind::Float { value: x }))
        } else {
            let i: i64 = cooked.parse().map_err(|_| self.err_at(start, "entier invalide"))?;
            Ok(self.mk_token(start, self.pos, TokenKind::Int { value: i }))
        }
    }

    fn collect_while<F: Fn(char)->bool>(&mut self, f: F) -> (String, bool) {
        let mut s = String::new(); let mut any=false;
        while let Some(c) = self.peek() {
            if f(c) { any=true; s.push(c); self.bump(); } else { break; }
        }
        (s, any)
    }

    /* ────── strings / bytes ────── */

    fn lex_string(&mut self, _raw: bool) -> Result<Token, LexError> {
        let start = self.pos;
        self.bump(); // opening "
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('"') => break,
                Some('\\') => {
                    let ch = self.parse_escape_char()?;
                    out.push(ch);
                }
                Some(c) => out.push(c),
                None => return Err(self.err_at(start, "chaîne non terminée")),
            }
        }
        Ok(self.mk_token(start, self.pos, TokenKind::Str { value: out }))
    }

    fn lex_bytes_literal(&mut self) -> Result<Token, LexError> {
        let start = self.pos; // on est après le 'b'
        // déjà validé: prochain char doit être '"'
        self.expect('"')?;
        let mut out = Vec::<u8>::new();
        loop {
            match self.bump() {
                Some('"') => break,
                Some('\\') => {
                    let b = self.parse_escape_byte()?;
                    out.push(b);
                }
                Some(c) => {
                    let mut buf = [0u8;4];
                    let s = c.encode_utf8(&mut buf);
                    out.extend_from_slice(s.as_bytes());
                }
                None => return Err(self.err_at(start, "b\"...\" non terminée")),
            }
        }
        Ok(self.mk_token(start, self.pos, TokenKind::Bytes { value: out }))
    }

    fn lex_hex_bytes_literal(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        // consomme 'hex"'
        self.bump(); self.bump(); self.bump(); // h e x
        self.expect('"')?;
        // lire jusqu'au prochain "
        let mut raw = String::new();
        loop {
            match self.bump() {
                Some('"') => break,
                Some(c) => raw.push(c),
                None => return Err(self.err_at(start, "hex\"...\" non terminée")),
            }
        }
        // parser
        let bytes = parse_hex_inline(&raw).map_err(|m| self.err_at(start, m))?;
        Ok(self.mk_token(start, self.pos, TokenKind::Bytes { value: bytes }))
    }

    fn parse_escape_char(&mut self) -> Result<char, LexError> {
        match self.bump() {
            Some('n')  => Ok('\n'),
            Some('t')  => Ok('\t'),
            Some('r')  => Ok('\r'),
            Some('\\') => Ok('\\'),
            Some('"')  => Ok('"'),
            Some('x')  => {
                let h1 = self.bump().ok_or_else(|| self.err_here("échappe \\x: manque 1 hex"))?;
                let h2 = self.bump().ok_or_else(|| self.err_here("échappe \\x: manque 1 hex"))?;
                let v = hex2(h1, h2).ok_or_else(|| self.err_here("échappe \\x: hex invalide"))?;
                Ok(v as char)
            }
            Some('u')  => {
                self.expect('{')?;
                let mut hex = String::new();
                let mut count = 0usize;
                while let Some(c) = self.peek() {
                    if c == '}' { self.bump(); break; }
                    if c.is_ascii_hexdigit() && count < 6 {
                        hex.push(c); self.bump(); count += 1;
                    } else {
                        return Err(self.err_here("échappe \\u{...} invalide"));
                    }
                }
                if hex.is_empty() { return Err(self.err_here("échappe \\u{...} vide")); }
                let code = u32::from_str_radix(&hex, 16).map_err(|_| self.err_here("codepoint unicode invalide"))?;
                let ch = char::from_u32(code).ok_or_else(|| self.err_here("codepoint hors plage"))?;
                Ok(ch)
            }
            Some(c) => Err(self.err_here(&format!("échappe inconnue: \\{c}"))),
            None => Err(self.err_here("échappe incomplète")),
        }
    }

    fn parse_escape_byte(&mut self) -> Result<u8, LexError> {
        match self.bump() {
            Some('n')  => Ok(b'\n'),
            Some('t')  => Ok(b'\t'),
            Some('r')  => Ok(b'\r'),
            Some('\\') => Ok(b'\\'),
            Some('"')  => Ok(b'"'),
            Some('0')  => Ok(0),
            Some('x')  => {
                let h1 = self.bump().ok_or_else(|| self.err_here("échappe \\x: manque 1 hex"))?;
                let h2 = self.bump().ok_or_else(|| self.err_here("échappe \\x: manque 1 hex"))?;
                hex2(h1, h2).ok_or_else(|| self.err_here("échappe \\x: hex invalide"))
            }
            Some(c) => {
                // fallback: 1er octet de l'UTF-8
                let mut buf = [0u8;4];
                let s = c.encode_utf8(&mut buf);
                Ok(s.as_bytes()[0])
            }
            None => Err(self.err_here("échappe incomplète")),
        }
    }

    fn expect(&mut self, ch: char) -> Result<(), LexError> {
        match self.bump() {
            Some(c) if c == ch => Ok(()),
            Some(_) => Err(self.err_here(&format!("attendu '{}'", ch))),
            None => Err(self.err_here(&format!("attendu '{}', trouvé EOF", ch))),
        }
    }

    /* ────── opérateurs / ponctuation ────── */

    fn lex_punct_or_op(&mut self) -> Result<Token, LexError> {
        macro_rules! two {
            ($a:literal, $kind:expr) => {{
                let start = self.pos; self.bump(); self.bump();
                Ok(self.mk_token(start, self.pos, $kind))
            }};
        }
        macro_rules! three {
            ($a:literal, $kind:expr) => {{
                let start = self.pos; self.bump(); self.bump(); self.bump();
                Ok(self.mk_token(start, self.pos, $kind))
            }};
        }

        // multi-caractères d'abord
        if self.starts_with("==") { return two!("==", TokenKind::EqEq); }
        if self.starts_with("!=") { return two!("!=", TokenKind::Ne); }
        if self.starts_with("<=") { return two!("<=", TokenKind::Le); }
        if self.starts_with(">=") { return two!(">=", TokenKind::Ge); }
        if self.starts_with("&&") { return two!("&&", TokenKind::AndAnd); }
        if self.starts_with("||") { return two!("||", TokenKind::OrOr); }
        if self.starts_with("->") { return two!("->", TokenKind::Arrow); }
        if self.starts_with("=>") { return two!("=>", TokenKind::FatArrow); }
        if self.starts_with("::") { return two!("::", TokenKind::ColonColon); }
        if self.starts_with("..="){ return three!("..=", TokenKind::RangeEq); }
        if self.starts_with("..."){ return three!("...", TokenKind::Ellipsis); }
        if self.starts_with("..") { return two!("..", TokenKind::Range); }
        if self.starts_with("+="){ return two!("+=", TokenKind::PlusAssign); }
        if self.starts_with("-="){ return two!("-=", TokenKind::MinusAssign); }
        if self.starts_with("*="){ return two!("*=", TokenKind::StarAssign); }
        if self.starts_with("/="){ return two!("/=", TokenKind::SlashAssign); }
        if self.starts_with("%="){ return two!("%=", TokenKind::PercentAssign); }
        if self.starts_with("&="){ return two!("&=", TokenKind::AndAssign); }
        if self.starts_with("|="){ return two!("|=", TokenKind::OrAssign); }
        if self.starts_with("^="){ return two!("^=", TokenKind::XorAssign); }
        if self.starts_with("<<="){ return three!("<<=", TokenKind::ShlAssign); }
        if self.starts_with(">>="){ return three!(">>=", TokenKind::ShrAssign); }
        if self.starts_with("<<"){ return two!("<<", TokenKind::Shl); }
        if self.starts_with(">>"){ return two!(">>", TokenKind::Shr); }

        // simples
        let start = self.pos;
        let c = self.bump().ok_or_else(|| self.err_here("caractère attendu"))?;
        let kind = match c {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            ';' => TokenKind::Semicolon,
            ':' => TokenKind::Colon,
            '?' => TokenKind::Question,
            '@' => TokenKind::At,
            '=' => TokenKind::Assign,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '&' => TokenKind::And,
            '|' => TokenKind::Or,
            '^' => TokenKind::Xor,
            '!' => TokenKind::Not,
            '<' => TokenKind::Lt,
            '>' => TokenKind::Gt,
            '~' => TokenKind::Tilde,
            '#' => TokenKind::Hash,
            other => {
                return Err(self.err_at(start, &format!("caractère inattendu: {:?}", other)));
            }
        };
        Ok(self.mk_token(start, self.pos, kind))
    }
}

/* ───────────────────────── Utils ident/nombres/hex ───────────────────────── */

fn is_ident_start(c: char) -> bool {
    // Heuristique "assez bonne" sans unicode-xid :
    c == '_' || c.is_ascii_alphabetic() || (c as u32) >= 0x80 && unicode_ident_start_friendly(c)
}
fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric() || (c as u32) >= 0x80 && unicode_ident_continue_friendly(c)
}

// Très grossier : autorise lettres/combining marks (~ cat L/M/Nd/Nl/No minimale)
fn unicode_ident_start_friendly(c: char) -> bool {
    c.is_alphabetic() || c == '·' // middle dot catalan, etc.
}
fn unicode_ident_continue_friendly(c: char) -> bool {
    unicode_ident_start_friendly(c) || c.is_ascii_digit() || c == ' ' // étendre au besoin
}

fn i64_from_base(raw: &str, base: u32) -> Option<i64> {
    // raw comme "+0xDEAD" / "-0b1010" / "0xFF" ; on nettoie
    let s: String = raw.chars().filter(|&c| c != '_' ).collect();
    let neg = s.starts_with('-');
    let trimmed = s.trim_start_matches(&['+','-'][..]);
    let digits = match base {
        16 => trimmed.trim_start_matches("0x").trim_start_matches("0X"),
        2  => trimmed.trim_start_matches("0b").trim_start_matches("0B"),
        _  => trimmed,
    };
    let v = i128::from_str_radix(digits, base).ok()?;
    let v = if neg { -v } else { v };
    i64::try_from(v).ok()
}

fn hex_val(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some((c as u8) - b'0'),
        'a'..='f' => Some((c as u8) - b'a' + 10),
        'A'..='F' => Some((c as u8) - b'A' + 10),
        _ => None,
    }
}
fn hex2(a: char, b: char) -> Option<u8> {
    Some(hex_val(a)? << 4 | hex_val(b)?)
}

/// Convertit `"DE AD BE EF"` (espaces et `_` tolérés) → bytes.
fn parse_hex_inline(s: &str) -> Result<Vec<u8>, &'static str> {
    let mut v = Vec::<u8>::new();
    let mut it = s.chars().filter(|c| !c.is_whitespace() && *c != '_');
    loop {
        let a = match it.next() { Some(c) => c, None => break };
        let b = it.next().ok_or("hex: nombre impair de nibbles")?;
        let byte = hex2(a, b).ok_or("hex: caractère non hexadécimal")?;
        v.push(byte);
    }
    Ok(v)
}

/* ───────────────────────── API top-level ───────────────────────── */

pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(src).tokenize_all()
}

/* ───────────────────────── Tests ───────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        tokenize(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn idents_keywords() {
        let v = kinds("let fn if else return true false null struct enum impl use as from in mut const pub mod extern abc αβγ");
        assert!(matches!(v[0], TokenKind::KwLet));
        assert!(matches!(v[1], TokenKind::KwFn));
        assert!(matches!(v[2], TokenKind::KwIf));
        assert!(matches!(v[5], TokenKind::KwTrue));
        assert!(matches!(v[6], TokenKind::KwFalse));
        assert!(matches!(v[7], TokenKind::KwNull));
        assert!(matches!(v.last().unwrap(), TokenKind::Ident(_)));
    }

    #[test]
    fn numbers_dec_hex_bin_float() {
        let t = tokenize("0 42 -7 1_000 0xDEAD_beef 0b1010_1111 3.14 .5 1. e-3 1_2.3_4e+5").unwrap();
        assert!(matches!(t[0].kind, TokenKind::Int{..}));
        assert!(matches!(t[1].kind, TokenKind::Int{..}));
        assert!(matches!(t[2].kind, TokenKind::Int{..}));
        assert!(matches!(t[3].kind, TokenKind::Int{..}));
        assert!(matches!(t[4].kind, TokenKind::Int{..}));
        assert!(matches!(t[5].kind, TokenKind::Int{..}));
        assert!(matches!(t[6].kind, TokenKind::Float{..}));
        assert!(matches!(t[7].kind, TokenKind::Float{..}));
        assert!(matches!(t[8].kind, TokenKind::Float{..}));
        assert!(matches!(t[9].kind, TokenKind::Float{..}));
    }

    #[test]
    fn strings_bytes_hex() {
        let t = tokenize(r#""hi\n" b"\xDE\xAD" hex"DE AD BE EF""#).unwrap();
        assert!(matches!(t[0].kind, TokenKind::Str{..}));
        assert!(matches!(t[1].kind, TokenKind::Bytes{..}));
        assert!(matches!(t[2].kind, TokenKind::Bytes{..}));
    }

    #[test]
    fn comments_nested() {
        let t = tokenize("/* a /* b */ c */ 42").unwrap();
        assert!(t.iter().any(|tk| matches!(tk.kind, TokenKind::Int{..})));
    }

    #[test]
    fn ops_and_punct() {
        let v = kinds("== != <= >= && || -> => :: .. ..= ... += -= *= /= %= &= |= ^= << >> <<=");
        use TokenKind::*;
        assert!(matches!(v[0], EqEq));
        assert!(matches!(v[1], Ne));
        assert!(matches!(v[2], Le));
        assert!(matches!(v[3], Ge));
        assert!(matches!(v[4], AndAnd));
        assert!(matches!(v[5], OrOr));
        assert!(matches!(v[6], Arrow));
        assert!(matches!(v[7], FatArrow));
        assert!(matches!(v[8], ColonColon));
        assert!(matches!(v[9], Range));
        assert!(matches!(v[10], RangeEq));
        assert!(matches!(v[11], Ellipsis));
        assert!(matches!(v[12], PlusAssign));
        assert!(matches!(v[13], MinusAssign));
        assert!(matches!(v[18], Shl));
        assert!(matches!(v[19], Shr));
        assert!(matches!(v[20], ShlAssign));
    }

    #[test]
    fn shebang_ok() {
        let v = kinds("#!/usr/bin/env vitte\nlet");
        assert!(matches!(v[0], TokenKind::KwLet));
    }
}
