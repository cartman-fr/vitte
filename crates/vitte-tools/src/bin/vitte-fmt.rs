// crates/vitte-tools/src/bin/vitte-fmt.rs
//! Formateur de code Vitte (.vit) — ultra complet mais sans prise de tête.
//!
//! Exemples :
//!   vitte-fmt src/main.vit --write
//!   vitte-fmt src/ --check --diff
//!   cat foo.vit | vitte-fmt - --stdin-name foo.vit --write -o out.vit
//!
//! Config (optionnelle) : ./.vittefmt.toml dans le projet ou parents :
//!   max_width = 100
//!   indent_width = 2
//!   use_tabs = false
//!   newline = "lf"        # "lf" | "crlf" | "native"
//!   space_around_ops = true
//!   trim_trailing = true
//!   ensure_final_newline = true
//!   collapse_blank_lines = 2
//!
//! Limites MVP : pas de “wrap” intelligent à max_width ; on normalise les espaces,
//! l’indentation, les commentaires et la ponctuation. Suffisant pour un style clean.

use std::cmp::min;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{ArgAction, Parser, ValueEnum};
use serde::Deserialize;
use yansi::{Paint, Color};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Newline {
    Lf,    // \n
    Crlf,  // \r\n
    Native,
}

impl Newline {
    fn as_str(self) -> &'static str {
        match self {
            Newline::Lf => "\n",
            Newline::Crlf => "\r\n",
            Newline::Native => if cfg!(windows) { "\r\n" } else { "\n" },
        }
    }
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default = "d_max_width")]
    max_width: usize,
    #[serde(default = "d_indent_width")]
    indent_width: usize,
    #[serde(default)]
    use_tabs: bool,
    #[serde(default = "d_newline")]
    newline: String,
    #[serde(default = "d_true")]
    space_around_ops: bool,
    #[serde(default = "d_true")]
    trim_trailing: bool,
    #[serde(default = "d_true")]
    ensure_final_newline: bool,
    #[serde(default = "d_collapse_blank")]
    collapse_blank_lines: usize,
}
fn d_max_width() -> usize { 100 }
fn d_indent_width() -> usize { 2 }
fn d_newline() -> String { "lf".into() }
fn d_true() -> bool { true }
fn d_collapse_blank() -> usize { 2 }

impl Default for Config {
    fn default() -> Self {
        Self {
            max_width: d_max_width(),
            indent_width: d_indent_width(),
            use_tabs: false,
            newline: d_newline(),
            space_around_ops: true,
            trim_trailing: true,
            ensure_final_newline: true,
            collapse_blank_lines: d_collapse_blank(),
        }
    }
}

impl Config {
    fn newline_mode(&self) -> Newline {
        match self.newline.as_str() {
            "lf" => Newline::Lf,
            "crlf" => Newline::Crlf,
            "native" => Newline::Native,
            _ => Newline::Lf,
        }
    }
    fn indent_unit(&self) -> String {
        if self.use_tabs { "\t".into() } else { " ".repeat(self.indent_width) }
    }
}

#[derive(Parser, Debug)]
#[command(name="vitte-fmt", version, about="Formateur de code Vitte (.vit)")]
struct Cli {
    /// Fichiers ou dossiers (ou '-' pour stdin unique)
    inputs: Vec<String>,

    /// Écrit les changements sur disque (in-place si fichier, ou --out/--out-dir)
    #[arg(long, action=ArgAction::SetTrue)]
    write: bool,

    /// Vérifie si les fichiers sont bien formatés (exit code ≠ 0 sinon)
    #[arg(long, action=ArgAction::SetTrue)]
    check: bool,

    /// Affiche un diff minimal (stdout) en mode --check
    #[arg(long, action=ArgAction::SetTrue)]
    diff: bool,

    /// Liste les fichiers qui seraient modifiés
    #[arg(long, action=ArgAction::SetTrue)]
    list: bool,

    /// Réduit la verbosité
    #[arg(long, action=ArgAction::SetTrue)]
    quiet: bool,

    /// Fichier de config .vittefmt.toml (sinon auto-discovery)
    #[arg(long)]
    config: Option<PathBuf>,

    /// Chemin de sortie si une seule entrée (sinon interdit ; utiliser --out-dir)
    #[arg(short, long, conflicts_with = "out_dir")]
    out: Option<PathBuf>,

    /// Dossier de sortie pour plusieurs entrées (garde <stem>.vit)
    #[arg(long)]
    out_dir: Option<PathBuf>,

    /// Nom logique pour stdin ('-')
    #[arg(long, default_value = "<stdin>")]
    stdin_name: String,

    /// Force un style de fin de ligne
    #[arg(long, value_enum)]
    newline: Option<Newline>,

    /// Largeur max (pas de wrap, mais utile pour futur)
    #[arg(long)]
    max_width: Option<usize>,

    /// Taille indentation (si pas de tabs)
    #[arg(long)]
    indent_width: Option<usize>,

    /// Utiliser des tabs pour indenter
    #[arg(long, action=ArgAction::SetTrue)]
    use_tabs: bool,
}

fn main() {
    if let Err(e) = real_main() {
        eprintln!("❌ {e}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    color_eyre::install().ok();

    let t0 = Instant::now();
    let mut cli = Cli::parse();

    if cli.inputs.is_empty() {
        return Err(anyhow!("Aucune entrée. Exemple: vitte-fmt src/ --check"));
    }
    let use_stdin = cli.inputs.len() == 1 && cli.inputs[0] == "-";
    if use_stdin && cli.out_dir.is_some() {
        return Err(anyhow!("`--out-dir` n’a pas de sens avec stdin ; utilise `--out`"));
    }
    if cli.inputs.len() > 1 && cli.out.is_some() {
        return Err(anyhow!("Plusieurs entrées → utilise `--out-dir` au lieu de `--out`"));
    }

    // Charge config
    let mut cfg = load_config(cli.config.as_deref().map(Path::new))
        .unwrap_or_default();

    // Overrides CLI
    if let Some(nl) = cli.newline { cfg.newline = match nl { Newline::Lf => "lf", Newline::Crlf => "crlf", Newline::Native => "native" }.into(); }
    if let Some(w) = cli.max_width { cfg.max_width = w; }
    if let Some(iw) = cli.indent_width { cfg.indent_width = iw; }
    if cli.use_tabs { cfg.use_tabs = true; }

    let mut total_changed = 0usize;
    let mut total_checked = 0usize;

    if use_stdin {
        let (src, name) = read_stdin(&cli.stdin_name)?;
        let (formatted, changed) = format_one(&src, &cfg)?;
        total_checked += 1;
        if changed {
            total_changed += 1;
            if cli.check && cli.diff {
                print_diff(&name, &src, &formatted, cfg.newline_mode().as_str());
            }
        }
        if cli.list && changed && !cli.quiet {
            eprintln!("{}", name);
        }

        if cli.write {
            if let Some(out) = &cli.out {
                write_text(Path::new(out), &formatted)?;
                if !cli.quiet { eprintln!("✓ Écrit → {}", Utf8PathBuf::from_path_buf(out.clone()).unwrap_or_else(|_| Utf8PathBuf::from("<out>"))); }
            } else {
                // in-place impossible sur stdin : écrire sur stdout
                print!("{formatted}");
            }
        } else if !cli.check {
            // par défaut : affiche sur stdout
            print!("{formatted}");
        }
    } else {
        // Parcours de tous les inputs (fichiers / répertoires)
        let mut files = Vec::<Utf8PathBuf>::new();
        for input in &cli.inputs {
            collect_vit_files(input, &mut files)?;
        }
        files.sort();

        for f in files {
            let src = fs::read_to_string(&f).with_context(|| format!("lecture {f}"))?;
            let (formatted, changed) = format_one(&src, &cfg)?;
            total_checked += 1;
            if changed {
                total_changed += 1;
                if cli.check && cli.diff {
                    print_diff(f.as_str(), &src, &formatted, cfg.newline_mode().as_str());
                }
            }

            if cli.list && changed && !cli.quiet {
                eprintln!("{f}");
            }

            if cli.write {
                let out_path = if let Some(out_dir) = &cli.out_dir {
                    let dir = Utf8PathBuf::from_path_buf(out_dir.clone()).map_err(|_| anyhow!("out-dir non UTF-8"))?;
                    let file = f.file_name().unwrap_or("out.vit");
                    dir.join(file)
                } else if let Some(out) = &cli.out {
                    if cli.inputs.len() > 1 { return Err(anyhow!("--out avec plusieurs entrées n’est pas autorisé.")); }
                    Utf8PathBuf::from_path_buf(out.clone()).map_err(|_| anyhow!("out non UTF-8"))?
                } else {
                    f.clone()
                };
                write_text(out_path.as_std_path(), &formatted)?;
                if !cli.quiet {
                    eprintln!("✓ Écrit → {}", out_path);
                }
            } else if !cli.check {
                // output to stdout only if a single file? – on s'abstient en multi
                if cli.inputs.len() == 1 && cli.out.is_none() && cli.out_dir.is_none() {
                    print!("{formatted}");
                }
            }
        }
    }

    // Bilan
    if cli.check {
        if total_changed > 0 {
            eprintln!("{}", format!("✗ {total_changed}/{total_checked} fichier(s) à reformater").paint(Color::Red));
            std::process::exit(2);
        } else if !cli.quiet {
            eprintln!("{}", format!("✓ {total_checked} fichier(s) OK").paint(Color::Green));
        }
    } else if !cli.quiet {
        eprintln!("⏱️  {}", human_millis(t0.elapsed()));
    }

    Ok(())
}

/* ----------------------------- I/O & Config ----------------------------- */

fn load_config(explicit: Option<&Path>) -> Result<Config> {
    if let Some(p) = explicit {
        let s = fs::read_to_string(p).with_context(|| format!("lecture config {}", p.display()))?;
        let cfg: Config = toml::from_str(&s).with_context(|| "TOML invalide")?;
        return Ok(cfg);
    }

    // recherche ascendante .vittefmt.toml
    let mut cur = std::env::current_dir()?;
    loop {
        let cand = cur.join(".vittefmt.toml");
        if cand.exists() {
            let s = fs::read_to_string(&cand).with_context(|| format!("lecture {}", cand.display()))?;
            let cfg: Config = toml::from_str(&s).with_context(|| "TOML invalide")?;
            return Ok(cfg);
        }
        if !cur.pop() { break; }
    }
    Ok(Config::default())
}

fn collect_vit_files(input: &str, out: &mut Vec<Utf8PathBuf>) -> Result<()> {
    let p = Utf8PathBuf::from(input);
    let md = fs::metadata(&p).with_context(|| format!("stat {p}"))?;
    if md.is_file() {
        if p.extension().map(|e| e.eq_ignore_ascii_case("vit")).unwrap_or(false) {
            out.push(p);
        }
        return Ok(());
    }
    // dir
    let mut stack = vec![p];
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(&dir)? {
            let e = e?;
            let path = Utf8PathBuf::from_path_buf(e.path()).map_err(|_| anyhow!("chemin non UTF-8"))?;
            if e.file_type()?.is_dir() {
                // ignore dossiers cachés type .git
                if !path.file_name().unwrap_or("").starts_with('.') {
                    stack.push(path);
                }
            } else if path.extension().map(|x| x.eq_ignore_ascii_case("vit")).unwrap_or(false) {
                out.push(path);
            }
        }
    }
    Ok(())
}

fn read_stdin(name: &str) -> Result<(String, String)> {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s)?;
    Ok((s, name.to_string()))
}

fn write_text(path: &Path, s: &str) -> Result<()> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let mut f = fs::File::create(path)?;
    f.write_all(s.as_bytes())?;
    Ok(())
}

/* ----------------------------- Lexer minimal ----------------------------- */

#[derive(Debug, Clone, PartialEq)]
enum TKind {
    Ident(String),
    Number(String),
    String(String),
    LineComment(String),   // // ...
    BlockComment(String),  // /* ... */
    Symbol(&'static str),  // punctuations & operators e.g. "==", "+", "{", "}"
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
struct Tok {
    kind: TKind,
}

struct Lexer<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self { Self { s: src.as_bytes(), i: 0 } }

    fn next_token(&mut self) -> TKind {
        self.skip_ws_except_nl();

        if self.eof() { return TKind::Eof; }

        // Newline
        if self.peek() == b'\n' {
            self.i += 1;
            return TKind::Newline;
        }
        if self.peek() == b'\r' && self.peek2() == Some(b'\n') {
            self.i += 2;
            return TKind::Newline;
        }

        // Comments
        if self.peek() == b'/' {
            if self.peek2() == Some(b'/') {
                self.i += 2;
                let start = self.i;
                while !self.eof() && self.peek() != b'\n' { self.i += 1; }
                let txt = String::from_utf8(self.s[start..self.i].to_vec()).unwrap_or_default();
                return TKind::LineComment(txt);
            }
            if self.peek2() == Some(b'*') {
                self.i += 2;
                let start = self.i;
                while !self.eof() && !(self.peek() == b'*' && self.peek2() == Some(b'/')) {
                    self.i += 1;
                }
                let txt = String::from_utf8(self.s[start..min(self.i, self.s.len())].to_vec()).unwrap_or_default();
                if !self.eof() { self.i += 2; }
                return TKind::BlockComment(txt);
            }
        }

        // String
        if self.peek() == b'"' {
            self.i += 1;
            let mut out = String::new();
            while !self.eof() {
                let c = self.peek();
                self.i += 1;
                match c {
                    b'"' => break,
                    b'\\' => {
                        if self.eof() { break; }
                        let e = self.peek();
                        self.i += 1;
                        match e {
                            b'n' => out.push('\n'),
                            b't' => out.push('\t'),
                            b'r' => out.push('\r'),
                            b'"' => out.push('"'),
                            b'\\' => out.push('\\'),
                            _ => { out.push('\\'); out.push(e as char); }
                        }
                    }
                    _ => out.push(c as char),
                }
            }
            return TKind::String(out);
        }

        // Number
        if self.peek().is_ascii_digit() {
            let start = self.i;
            self.i += 1;
            while !self.eof() && (self.peek().is_ascii_digit() || self.peek() == b'.') {
                self.i += 1;
            }
            let txt = String::from_utf8(self.s[start..self.i].to_vec()).unwrap_or_default();
            return TKind::Number(txt);
        }

        // Ident
        if is_ident_start(self.peek() as char) {
            let start = self.i;
            self.i += 1;
            while !self.eof() && is_ident_continue(self.peek() as char) {
                self.i += 1;
            }
            let txt = String::from_utf8(self.s[start..self.i].to_vec()).unwrap_or_default();
            return TKind::Ident(txt);
        }

        // Operators & symbols (2-char first)
        for &two in &["==","!=", "<=", ">=", "&&", "||", "::"] {
            if self.starts_with(two.as_bytes()) {
                self.i += 2;
                return TKind::Symbol(two);
            }
        }
        // Single-char
        let ch = self.peek() as char;
        self.i += 1;
        let sym = match ch {
            '{'|'}'|'(' |')'|'['|']'|
            ';'|','|'.'|
            '+'|'-'|'*'|'/'|'%'|
            '<'|'>'|'!'|'='|':' => {
                let mut s = [0u8; 1]; s[0] = ch as u8;
                let owned = String::from_utf8_lossy(&s).to_string();
                // SAFETY: map to &'static str via leak (acceptable in tool)
                Box::leak(owned.into_boxed_str())
            }
            _ => "",
        };
        if sym.is_empty() { return TKind::Symbol(""); }
        TKind::Symbol(sym)
    }

    fn lex(mut self) -> Vec<Tok> {
        let mut v = Vec::new();
        loop {
            let k = self.next_token();
            let end = matches!(k, TKind::Eof);
            v.push(Tok { kind: k });
            if end { break; }
        }
        v
    }

    fn skip_ws_except_nl(&mut self) {
        while !self.eof() {
            let c = self.peek();
            if c == b' ' || c == b'\t' { self.i += 1; }
            else { break; }
        }
    }

    fn starts_with(&self, b: &[u8]) -> bool {
        self.s.get(self.i..self.i + b.len()).map(|x| x == b).unwrap_or(false)
    }
    fn peek(&self) -> u8 { self.s[self.i] }
    fn peek2(&self) -> Option<u8> { self.s.get(self.i+1).cloned() }
    fn eof(&self) -> bool { self.i >= self.s.len() }
}

fn is_ident_start(c: char) -> bool { c.is_ascii_alphabetic() || c == '_' }
fn is_ident_continue(c: char) -> bool { c.is_ascii_alphanumeric() || c == '_' }

/* ----------------------------- Formatter ----------------------------- */

fn format_one(src: &str, cfg: &Config) -> Result<(String, bool)> {
    let mut tokens = Lexer::new(src).lex();
    // Retire le dernier EOF token signifiant
    if let Some(last) = tokens.last() {
        if matches!(last.kind, TKind::Eof) { tokens.pop(); }
    }

    let nl = cfg.newline_mode().as_str();
    let indent_unit = cfg.indent_unit();

    let mut out = String::with_capacity(src.len() + src.len()/20);
    let mut indent: i32 = 0;
    let mut at_line_start = true;
    let mut blank_run = 0usize;

    // Petite vue sur le token suivant
    for i in 0..tokens.len() {
        use TKind::*;
        let tk = &tokens[i];
        let next = tokens.get(i+1).map(|t| &t.kind);

        match &tk.kind {
            Newline => {
                // compress blank lines
                blank_run += 1;
                continue;
            }
            LineComment(text) => {
                // force newline before un commentaire si on n'est pas au début
                if !at_line_start { out.push_str(nl); }
                write_indent(&mut out, indent, &indent_unit);
                out.push_str("//");
                out.push_str(text.trim_end());
                out.push_str(nl);
                at_line_start = true;
                blank_run = 0;
            }
            BlockComment(text) => {
                if !at_line_start { out.push_str(nl); }
                write_indent(&mut out, indent, &indent_unit);
                out.push_str("/*");
                out.push_str(text);
                out.push_str("*/");
                out.push_str(nl);
                at_line_start = true;
                blank_run = 0;
            }
            Symbol("{") => {
                if !at_line_start {
                    out.push_str(" ");
                }
                out.push('{'); out.push_str(nl);
                indent += 1;
                write_indent(&mut out, indent, &indent_unit);
                at_line_start = true;
                blank_run = 0;
            }
            Symbol("}") => {
                // fermer le bloc
                indent = (indent - 1).max(0);
                if !at_line_start { out.push_str(nl); }
                write_indent(&mut out, indent, &indent_unit);
                out.push('}');
                // newline si prochain token pas ';' ni ',' ni operator binaire
                if !matches!(next, Some(Symbol(";"))) {
                    out.push_str(nl);
                    at_line_start = true;
                } else {
                    at_line_start = false;
                }
                blank_run = 0;
            }
            Symbol(";") => {
                out.push(';');
                out.push_str(nl);
                at_line_start = true;
                blank_run = 0;
            }
            Symbol(",") => {
                out.push(',');
                if !matches!(next, Some(Symbol(")")) | Some(Symbol("]")) | Some(Symbol("}"))) {
                    out.push(' ');
                }
                at_line_start = false;
                blank_run = 0;
            }
            Symbol("(") => {
                if !at_line_start && needs_space_before_lparen(&tokens, i) {
                    out.push(' ');
                }
                out.push('(');
                at_line_start = false;
                blank_run = 0;
            }
            Symbol(")") => { out.push(')'); at_line_start = false; blank_run = 0; }
            Symbol("[") => { out.push('['); at_line_start = false; blank_run = 0; }
            Symbol("]") => { out.push(']'); at_line_start = false; blank_run = 0; }
            Symbol(sym) if is_operator(sym) => {
                let unary = is_unary_op(&tokens, i);
                if cfg.space_around_ops && !unary {
                    out.push(' ');
                }
                out.push_str(sym);
                if cfg.space_around_ops && !unary {
                    out.push(' ');
                }
                at_line_start = false;
                blank_run = 0;
            }
            Ident(s) => {
                if at_line_start {
                    write_indent(&mut out, indent, &indent_unit);
                } else if needs_space_before_ident(&tokens, i) {
                    out.push(' ');
                }
                out.push_str(s);
                at_line_start = false;
                blank_run = 0;
            }
            Number(n) => {
                if at_line_start { write_indent(&mut out, indent, &indent_unit); }
                else { out.push(' '); }
                out.push_str(n);
                at_line_start = false;
                blank_run = 0;
            }
            String(s) => {
                if at_line_start { write_indent(&mut out, indent, &indent_unit); }
                else { out.push(' '); }
                out.push('"');
                out.push_str(&escape_string_for_emit(s));
                out.push('"');
                at_line_start = false;
                blank_run = 0;
            }
            Symbol(sym) if !sym.is_empty() => {
                if at_line_start { write_indent(&mut out, indent, &indent_unit); }
                out.push_str(sym);
                at_line_start = false;
                blank_run = 0;
            }
            _ => {}
        }

        // Gérer éventuel bloc de lignes vides accumulées, à la limite fixée
        if at_line_start && blank_run > 0 {
            let keep = min(blank_run, cfg.collapse_blank_lines);
            for _ in 0..keep {
                out.push_str(nl);
            }
            blank_run = 0;
        }
    }

    // Trim trailing spaces et s’assurer d’un EOL final
    if cfg.trim_trailing {
        out = trim_trailing_spaces_lines(&out, nl);
    }
    if cfg.ensure_final_newline && !out.ends_with(nl) {
        out.push_str(nl);
    }

    let changed = normalize_newlines(src) != normalize_newlines(&out);
    Ok((out, changed))
}

fn write_indent(out: &mut String, indent: i32, unit: &str) {
    for _ in 0..indent { out.push_str(unit); }
}

fn escape_string_for_emit(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}

fn is_operator(sym: &str) -> bool {
    matches!(sym,
        "+"|"-"|"*"|"/"|"%"|
        "=="|"!="|"<"|">"|"<="|">="|
        "&&"|"||"|"!"|"="|":"
    )
}

fn is_unary_op(tokens: &[Tok], i: usize) -> bool {
    let sym = match &tokens[i].kind { TKind::Symbol(s) => *s, _ => return false };
    if sym != "-" && sym != "!" { return false; }
    // heuristique : si précédent est un début d'expression ou un operator, alors c'est unaire
    let prev = tokens.get(i.wrapping_sub(1)).map(|t| &t.kind);
    match prev {
        None => true,
        Some(TKind::Symbol(s)) if *s == "(" || *s == "[" || *s == "{" || is_operator(s) || *s == "," || *s == ";" => true,
        Some(TKind::Newline) => true,
        _ => false,
    }
}

fn needs_space_before_ident(tokens: &[Tok], i: usize) -> bool {
    if i == 0 { return false; }
    match &tokens[i-1].kind {
        TKind::Ident(_) | TKind::Number(_) | TKind::String(_) | TKind::Symbol(")") | TKind::Symbol("]") => true,
        TKind::Symbol(sym) if sym == "}" => true,
        _ => false,
    }
}

fn needs_space_before_lparen(tokens: &[Tok], i: usize) -> bool {
    if i == 0 { return false; }
    match &tokens[i-1].kind {
        TKind::Ident(_) | TKind::Symbol(")") | TKind::Symbol("]") => true,
        _ => false,
    }
}

fn trim_trailing_spaces_lines(s: &str, nl: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for line in s.split_inclusive(nl) {
        let mut l = line.to_string();
        if l.ends_with(nl) {
            let base = &l[..l.len()-nl.len()];
            let trimmed = base.trim_end_matches(|c: char| c == ' ' || c == '\t');
            l = format!("{trimmed}{nl}");
        }
        out.push_str(&l);
    }
    out
}

fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n")
}

/* ----------------------------- Diff (simple) ----------------------------- */

fn print_diff(name: &str, old: &str, new: &str, nl: &str) {
    let oldl: Vec<&str> = normalize_newlines(old).split('\n').collect();
    let newl: Vec<&str> = normalize_newlines(new).split('\n').collect();

    eprintln!("{}", format!("--- {name}").paint(Color::Red));
    eprintln!("{}", format!("+++ {name} (formatted)").paint(Color::Green));

    let mut i = 0usize;
    let mut j = 0usize;
    while i < oldl.len() || j < newl.len() {
        if i < oldl.len() && j < newl.len() && oldl[i] == newl[j] {
            i += 1; j += 1;
            continue;
        }
        // lignes supprimées
        if i < oldl.len() && (j >= newl.len() || !newl[j..].contains(&oldl[i])) {
            eprintln!("{}", format!("-{}", oldl[i]).paint(Color::Red));
            i += 1;
            continue;
        }
        // lignes ajoutées
        if j < newl.len() && (i >= oldl.len() || !oldl[i..].contains(&newl[j])) {
            eprintln!("{}", format!("+{}", newl[j]).paint(Color::Green));
            j += 1;
            continue;
        }
        // fallback : montrer paires
        if i < oldl.len() {
            eprintln!("{}", format!("-{}", oldl[i]).paint(Color::Red));
            i += 1;
        }
        if j < newl.len() {
            eprintln!("{}", format!("+{}", newl[j]).paint(Color::Green));
            j += 1;
        }
    }

    // ensure end newline for cleanliness when printing
    let _ = nl;
}

/* ----------------------------- Utils ----------------------------- */

fn human_millis(d: std::time::Duration) -> String {
    let ms = d.as_millis();
    if ms < 1_000 { return format!("{ms} ms"); }
    let s = ms as f64 / 1000.0;
    if s < 60.0 { return format!("{s:.3} s"); }
    let m = (s / 60.0).floor();
    let rest = s - m * 60.0;
    format!("{m:.0} min {rest:.1} s")
}
