//! runtime/parser.rs — Parseur de littéraux vers `ConstValue` (sans deps)
//!
//! Pris en charge :
//!   null
//!   true | false
//!   entiers i64      : 42, -7, 1_000_000, 0xDEAD_BEEF, 0b1010_1111
//!   flottants f64    : 3.14, -2.5e-3, 1_000.5
//!   chaînes          : "hello\nworld", échappes: \n \t \r \\ \" \xHH \u{...}
//!   octets (bytes)   : b"raw\0bytes", hex"DE AD BE EF"
//!
//! API :
//!   - parse_value(input) -> ConstValue
//!   - parse_list(input)  -> Vec<ConstValue>   // "a, 1, null" ou "[a, 1, null]" (a = "a")
//!   - try_number(str)    -> Option<ConstValue>
//!
//! Exemples :
//! ```
//! use vitte_core::bytecode::chunk::ConstValue;
//! use vitte_core::runtime::parser::parse_value;
//! assert!(matches!(parse_value("null").unwrap(), ConstValue::Null));
//! assert!(matches!(parse_value("true").unwrap(), ConstValue::Bool(true)));
//! assert!(matches!(parse_value("0xFF").unwrap(), ConstValue::I64(255)));
//! assert!(matches!(parse_value("3.5e1").unwrap(), ConstValue::F64(x) if (x-35.0).abs()<1e-12));
//! assert!(matches!(parse_value(r#""hi\n""#).unwrap(), ConstValue::Str(s) if s=="hi\n"));
//! assert!(matches!(parse_value(r#"b"\xDE\xAD""#).unwrap(), ConstValue::Bytes(v) if v==vec![0xDE,0xAD]));
//! assert!(matches!(parse_value(r#"hex"DE AD BE EF""#).unwrap(), ConstValue::Bytes(v) if v==vec![0xDE,0xAD,0xBE,0xEF]));
//! ```

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, unused_must_use)]

use crate::bytecode::chunk::ConstValue;

/* ───────────────────────────── Erreur & Span ───────────────────────────── */

/// Position (ligne/col, 1-based) pour des erreurs lisibles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos { pub line: u32, pub col: u32 }

impl Pos {
    fn start() -> Self { Self { line: 1, col: 1 } }
    fn advance(&mut self, ch: char) {
        if ch == '\n' { self.line += 1; self.col = 1; } else { self.col += 1; }
    }
}

/// Erreur de parsing (message + position).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub pos: Pos,
    pub msg: String,
}
impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} (line {}, col {})", self.msg, self.pos.line, self.pos.col)
    }
}
impl std::error::Error for ParseError {}

/* ───────────────────────────── API publique ───────────────────────────── */

/// Parse **un** littéral en `ConstValue`.
pub fn parse_value(input: &str) -> Result<ConstValue, ParseError> {
    let mut p = Parser::new(input);
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if !p.eof() {
        return Err(p.err_here("caractères superflus après le littéral"));
    }
    Ok(v)
}

/// Parse une **liste** comma-séparée de littéraux en `Vec<ConstValue>`.
/// Accepte aussi la variante entre crochets : `[a, 1, null]`.
pub fn parse_list(input: &str) -> Result<Vec<ConstValue>, ParseError> {
    let mut p = Parser::new(input);
    p.skip_ws();
    let bracketed = p.peek_is('[');
    if bracketed { p.bump_char(); p.skip_ws(); }
    let mut out = Vec::new();
    if bracketed && p.peek_is(']') {
        p.bump_char();
        return Ok(out);
    }
    if !p.eof() {
        loop {
            let v = p.parse_value()?;
            out.push(v);
            p.skip_ws();
            if p.peek_is(',') { p.bump_char(); p.skip_ws(); continue; }
            break;
        }
    }
    if bracketed {
        p.expect_char(']')?;
    } else {
        p.skip_ws();
        if !p.eof() {
            return Err(p.err_here("caractères superflus après la liste"));
        }
    }
    Ok(out)
}

/// Essaie d’interpréter rapidement une chaîne comme **nombre**.
/// Renvoie `Some(ConstValue::I64/F64)` si ok, sinon `None`.
pub fn try_number(s: &str) -> Option<ConstValue> {
    let mut p = Parser::new(s);
    p.skip_ws();
    p.parse_number().ok()
}

/* ───────────────────────────── Impl du parseur ───────────────────────────── */

struct Parser<'a> {
    src: &'a str,
    it: std::str::Chars<'a>,
    look: Option<char>,
    pos: Pos,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        let mut it = src.chars();
        let look = it.next();
        Self { src, it, look, pos: Pos::start() }
    }

    /* ----- cursor ----- */

    fn eof(&self) -> bool { self.look.is_none() }
    fn peek(&self) -> Option<char> { self.look }
    fn peek_is(&self, ch: char) -> bool { self.look == Some(ch) }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.look?;
        self.pos.advance(ch);
        self.look = self.it.next();
        Some(ch)
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() { self.bump_char(); } else { break; }
        }
    }

    fn expect_char(&mut self, ch: char) -> Result<(), ParseError> {
        match self.bump_char() {
            Some(c) if c == ch => Ok(()),
            Some(_) => Err(self.err_here(&format!("attendu '{}'", ch))),
            None => Err(self.err_here(&format!("attendu '{}', trouvé EOF", ch))),
        }
    }

    fn err_here(&self, msg: &str) -> ParseError { ParseError { pos: self.pos, msg: msg.to_string() } }

    /* ----- parseurs ----- */

    fn parse_value(&mut self) -> Result<ConstValue, ParseError> {
        self.skip_ws();
        match self.peek() {
            None => Err(self.err_here("littéral attendu, trouvé EOF")),
            Some('"') => {
                let s = self.parse_string(false)?;
                Ok(ConstValue::Str(s))
            }
            Some('b') => {
                // b"..."  => bytes
                // hex"...." => bytes hex
                // ou ident bool:null au pire (boulon) -> géré plus bas
                if self.starts_with("b\"") {
                    let _ = self.bump_char(); // b
                    let _ = self.expect_char('"')?;
                    let bytes = self.parse_string_bytes_body()?;
                    self.expect_char('"')?;
                    Ok(ConstValue::Bytes(bytes))
                } else if self.starts_with("bytes\"") {
                    for _ in 0..5 { let _ = self.bump_char(); } // consume 'bytes'
                    self.expect_char('"')?;
                    let bytes = self.parse_string_bytes_body()?;
                    self.expect_char('"')?;
                    Ok(ConstValue::Bytes(bytes))
                } else {
                    // peut-être un ident débutant par b ? => passe à parse_ident/number
                    self.parse_number_or_ident()
                }
            }
            Some('h') => {
                if self.starts_with("hex\"") {
                    for _ in 0..3 { let _ = self.bump_char(); } // consume 'hex'
                    self.expect_char('"')?;
                    let txt = self.parse_until_quote()?;
                    self.expect_char('"')?;
                    let bytes = parse_hex_inline(&txt).map_err(|e| self.err_here(e))?;
                    Ok(ConstValue::Bytes(bytes))
                } else {
                    self.parse_number_or_ident()
                }
            }
            Some(c) if c == '-' || c == '+' || c.is_ascii_digit() => self.parse_number(),
            Some(_) => self.parse_number_or_ident(),
        }
    }

    fn parse_number_or_ident(&mut self) -> Result<ConstValue, ParseError> {
        // ident : null | true | false
        if self.starts_with("null") {
            for _ in 0..4 { let _ = self.bump_char(); }
            return Ok(ConstValue::Null);
        }
        if self.starts_with("true") {
            for _ in 0..4 { let _ = self.bump_char(); }
            return Ok(ConstValue::Bool(true));
        }
        if self.starts_with("false") {
            for _ in 0..5 { let _ = self.bump_char(); }
            return Ok(ConstValue::Bool(false));
        }
        // nombre
        self.parse_number()
    }

    fn parse_number(&mut self) -> Result<ConstValue, ParseError> {
        // Grammaire tolérante :
        //   [+-]? ( 0x[0-9A-Fa-f][0-9A-Fa-f_]* | 0b[01][01_]* | digits[digits_]* ('.' digits+)? (exp)? )
        // exp := [eE] [+-]? digits+
        // underscores autorisés dans toutes les variantes (supprimés)
        self.skip_ws();
        let start_pos = self.pos;

        // Colle les caractères potentiellement numériques dans un buffer
        let mut buf = String::new();

        // signe
        if matches!(self.peek(), Some('+') | Some('-')) {
            buf.push(self.bump_char().unwrap());
        }

        // 0x / 0b ?
        if self.starts_with("0x") || self.starts_with("0X") {
            buf.push_str("0x"); self.bump_char(); self.bump_char();
            // au moins un hex digit
            let mut any = false;
            while let Some(c) = self.peek() {
                if c == '_' { self.bump_char(); continue; }
                if c.is_ascii_hexdigit() { any=true; buf.push(self.bump_char().unwrap()); }
                else { break; }
            }
            if !any { return Err(ParseError { pos: start_pos, msg: "hexadécimal: chiffre attendu".into() }); }
            // parse i64
            let s = buf.replace('_', "");
            let val = i64::from_str_radix(s.trim_start_matches(&['+','-','0','x','X'][..]), 16)
                .map_err(|_| ParseError { pos: start_pos, msg: "entier hex invalide".into() })?;
            let sign = if s.starts_with('-') { -1 } else { 1 };
            return Ok(ConstValue::I64(sign * (val as i64)));
        }

        if self.starts_with("0b") || self.starts_with("0B") {
            buf.push_str("0b"); self.bump_char(); self.bump_char();
            let mut any = false;
            while let Some(c) = self.peek() {
                if c == '_' { self.bump_char(); continue; }
                if c == '0' || c == '1' { any=true; buf.push(self.bump_char().unwrap()); }
                else { break; }
            }
            if !any { return Err(ParseError { pos: start_pos, msg: "binaire: chiffre attendu".into() }); }
            let s = buf.replace('_', "");
            let val = i64::from_str_radix(s.trim_start_matches(&['+','-','0','b','B'][..]), 2)
                .map_err(|_| ParseError { pos: start_pos, msg: "entier binaire invalide".into() })?;
            let sign = if s.starts_with('-') { -1 } else { 1 };
            return Ok(ConstValue::I64(sign * (val as i64)));
        }

        // décimal / float
        let mut any_digit = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() { any_digit=true; buf.push(self.bump_char().unwrap()); }
            else if c == '_' { self.bump_char(); }
            else { break; }
        }

        let mut is_float = false;

        // point fractionnel
        if self.peek_is('.') {
            // Regarder si c’est un '.' suivi d’un digit → float ; sinon, nombre entier terminé.
            let mut clone = self.clone();
            let _ = clone.bump_char(); // '.'
            if matches!(clone.peek(), Some(d) if d.is_ascii_digit()) {
                is_float = true;
                buf.push('.'); self.bump_char(); // consomme '.'
                let mut any = false;
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() { any=true; buf.push(self.bump_char().unwrap()); }
                    else if c == '_' { self.bump_char(); }
                    else { break; }
                }
                if !any { return Err(ParseError { pos: start_pos, msg: "chiffres attendus après '.'".into() }); }
            }
        }

        // exposant ?
        if matches!(self.peek(), Some('e') | Some('E')) {
            is_float = true;
            buf.push(self.bump_char().unwrap()); // e/E
            if matches!(self.peek(), Some('+') | Some('-')) {
                buf.push(self.bump_char().unwrap());
            }
            let mut any = false;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() { any=true; buf.push(self.bump_char().unwrap()); }
                else if c == '_' { self.bump_char(); }
                else { break; }
            }
            if !any { return Err(ParseError { pos: start_pos, msg: "exposant: chiffre attendu".into() }); }
        }

        if !any_digit && !is_float {
            return Err(ParseError { pos: start_pos, msg: "nombre attendu".into() });
        }

        // Nettoyage underscores
        let s_clean: String = buf.chars().filter(|&c| c != '_').collect();

        if is_float || s_clean.contains('.') || s_clean.contains('e') || s_clean.contains('E') {
            let x = s_clean.parse::<f64>().map_err(|_| ParseError { pos: start_pos, msg: "flottant invalide".into() })?;
            Ok(ConstValue::F64(x))
        } else {
            let i = s_clean.parse::<i64>().map_err(|_| ParseError { pos: start_pos, msg: "entier invalide".into() })?;
            Ok(ConstValue::I64(i))
        }
    }

    fn parse_string(&mut self, bytes: bool) -> Result<String, ParseError> {
        // on s’attend à être sur '"'
        self.expect_char('"')?;
        let mut out = String::new();
        loop {
            match self.bump_char() {
                Some('"') => break,
                Some('\\') => {
                    let ch = self.parse_escape(bytes)?;
                    out.push(ch);
                }
                Some(c) => out.push(c),
                None => return Err(self.err_here("chaine non terminée")),
            }
        }
        Ok(out)
    }

    /// Corps d’une chaîne **déjà après** l’ouverture `"`, pour b"..." (renvoie bytes).
    fn parse_string_bytes_body(&mut self) -> Result<Vec<u8>, ParseError> {
        let mut out = Vec::<u8>::new();
        loop {
            match self.bump_char() {
                Some('"') => { self.unread('"'); break; } // remit le guillemet pour le *caller*
                Some('\\') => {
                    let b = self.parse_escape_byte()?;
                    out.push(b);
                }
                Some(c) => {
                    let mut buf = [0u8; 4];
                    let enc = c.encode_utf8(&mut buf);
                    out.extend_from_slice(enc.as_bytes());
                }
                None => return Err(self.err_here("chaine b\"...\" non terminée")),
            }
        }
        Ok(out)
    }

    /// Lecture jusqu’au prochain `"`, sans interpréter les échappes (pour hex"…")
    fn parse_until_quote(&mut self) -> Result<String, ParseError> {
        let mut out = String::new();
        loop {
            match self.bump_char() {
                Some('"') => { self.unread('"'); break; }
                Some(c) => out.push(c),
                None => return Err(self.err_here("chaine hex\"...\" non terminée")),
            }
        }
        Ok(out)
    }

    fn unread(&mut self, ch: char) {
        // petit “pushback” (sécurisé car nous ne faisons qu’une étape de lookahead)
        self.look = Some(ch);
        // Nous n’ajustons pas pos ici (non critique, l’appelant consommera juste après).
    }

    fn parse_escape(&mut self, _bytes: bool) -> Result<char, ParseError> {
        match self.bump_char() {
            Some('n')  => Ok('\n'),
            Some('t')  => Ok('\t'),
            Some('r')  => Ok('\r'),
            Some('\\') => Ok('\\'),
            Some('"')  => Ok('"'),
            Some('x')  => {
                let h1 = self.bump_char().ok_or_else(|| self.err_here("échappe \\x: hex manquant"))?;
                let h2 = self.bump_char().ok_or_else(|| self.err_here("échappe \\x: hex manquant"))?;
                let v = hex2(h1, h2).ok_or_else(|| self.err_here("échappe \\x: hex invalide"))?;
                Ok(v as char)
            }
            Some('u')  => {
                // \u{H..}  (1..6 hex)
                self.expect_char('{')?;
                let mut hex = String::new();
                let mut count = 0usize;
                while let Some(c) = self.peek() {
                    if c == '}' { self.bump_char(); break; }
                    if c.is_ascii_hexdigit() && count < 6 {
                        hex.push(self.bump_char().unwrap());
                        count += 1;
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

    fn parse_escape_byte(&mut self) -> Result<u8, ParseError> {
        match self.bump_char() {
            Some('n')  => Ok(b'\n'),
            Some('t')  => Ok(b'\t'),
            Some('r')  => Ok(b'\r'),
            Some('\\') => Ok(b'\\'),
            Some('"')  => Ok(b'"'),
            Some('0')  => Ok(0),
            Some('x')  => {
                let h1 = self.bump_char().ok_or_else(|| self.err_here("échappe \\x: hex manquant"))?;
                let h2 = self.bump_char().ok_or_else(|| self.err_here("échappe \\x: hex manquant"))?;
                hex2(h1, h2).ok_or_else(|| self.err_here("échappe \\x: hex invalide"))
            }
            Some(c) => {
                // Fallback: prends l’UTF-8 du char
                let mut buf = [0u8; 4];
                let enc = c.encode_utf8(&mut buf);
                Ok(enc.as_bytes()[0]) // note: ne gère pas multibyte entièrement, mais souvent suffisant
            }
            None => Err(self.err_here("échappe incomplète")),
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src[self.offset()..].starts_with(s)
    }

    fn offset(&self) -> usize {
        // Estimation de l’offset courant via différence de longueur (évite d’entreposer l’index).
        let consumed = self.it.as_str().len();
        self.src.len() - (consumed + self.look.map(|c| c.len_utf8()).unwrap_or(0))
    }
}

/* ───────────────────────────── Utilitaires locaux ───────────────────────────── */

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

/// Convertit une chaîne `"DE AD BE EF"` en bytes (espaces et `_` tolérés).
fn parse_hex_inline(s: &str) -> Result<Vec<u8>, &'static str> {
    let mut v = Vec::<u8>::new();
    let mut it = s.chars().filter(|c| !c.is_whitespace() && *c != '_');
    loop {
        let a = match it.next() {
            Some(c) => c,
            None => break,
        };
        let b = it.next().ok_or("hex: nombre impair de nibbles")?;
        let byte = hex2(a, b).ok_or("hex: caractère non hexadécimal")?;
        v.push(byte);
    }
    Ok(v)
}

/* ───────────────────────────── Tests ───────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn null_bool() {
        assert!(matches!(parse_value("null").unwrap(), ConstValue::Null));
        assert!(matches!(parse_value("true").unwrap(), ConstValue::Bool(true)));
        assert!(matches!(parse_value("false").unwrap(), ConstValue::Bool(false)));
    }
    #[test] fn ints_bases() {
        assert!(matches!(parse_value("0").unwrap(), ConstValue::I64(0)));
        assert!(matches!(parse_value("42").unwrap(), ConstValue::I64(42)));
        assert!(matches!(parse_value("-7").unwrap(), ConstValue::I64(-7)));
        assert!(matches!(parse_value("1_000_000").unwrap(), ConstValue::I64(1_000_000)));
        assert!(matches!(parse_value("0xDE_AD_be_ef").unwrap(), ConstValue::I64(i) if i == 0xDEAD_BEEF));
        assert!(matches!(parse_value("0b1010_1111").unwrap(), ConstValue::I64(i) if i == 0b1010_1111));
    }
    #[test] fn floats() {
        assert!(matches!(parse_value("3.14").unwrap(), ConstValue::F64(x) if (x-3.14).abs()<1e-12));
        assert!(matches!(parse_value("-2.5e-3").unwrap(), ConstValue::F64(x) if (x+0.0025).abs()<1e-12));
        assert!(try_number("  +10 ").is_some());
    }
    #[test] fn strings_and_bytes() {
        assert!(matches!(parse_value(r#""hi\n\t\r \\\"""#).unwrap(), ConstValue::Str(s) if s=="hi\n\t\r \\\""));
        assert!(matches!(parse_value(r#"b"\xDE\xAD \x00""#).unwrap(), ConstValue::Bytes(b) if b==vec![0xDE,0xAD,0x20,0x00]));
        assert!(matches!(parse_value(r#"hex"DE AD BE EF""#).unwrap(), ConstValue::Bytes(b) if b==vec![0xDE,0xAD,0xBE,0xEF]));
    }
    #[test] fn lists() {
        let v = parse_list(r#" [ null , "a" , 1 , 2.0 , b"\xFF" , hex"00 ff" ] "#).unwrap();
        assert_eq!(v.len(), 6);
    }
    #[test] fn trailing_garbage_err() {
        let e = parse_value("42 xyz").unwrap_err();
        assert!(e.msg.contains("superflus"));
    }
}
