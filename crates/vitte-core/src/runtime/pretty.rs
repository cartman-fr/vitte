//! pretty.rs — Outils d’affichage “jolis” pour Vitte (sans dépendances)
//!
//! ✨ Ce module fournit :
//! - `PrettyOptions` : options (couleur, tailles max, etc.)
//! - `paint*` : helpers ANSI (facultatif, togglé par `PrettyOptions.color`)
//! - `escape_str`, `preview_str`, `preview_bytes`, `hexdump`
//! - `pretty_value` / `pretty_const` : mise en forme des `ConstValue`
//! - `pretty_op_line` : une ligne d’op avec annotations (const preview, sauts → labels)
//! - `pretty_chunk_header`, `pretty_const_pool_table`, `pretty_line_table`,
//!   `pretty_debug_info`, `pretty_code_listing`
//! - `pretty_chunk_report(title, &chunk, &opts)` : rapport complet clé-en-main
//!
//! Le module ne dépend que de `std`. Il complète `disasm.rs` en offrant
//! une variante *colorée* et “tableau” réutilisable côté CLI/outils.
//!
//! Bonus : fonctions de tableau (`tabrow`, `tabsep`) et palette sobre.

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, unused_must_use)]

use std::fmt::Write as _;
use std::cmp::max;

use crate::bytecode::{
    chunk::{Chunk, ConstValue},
    op::Op,
};

/* ───────────────────────────── Options & couleurs ───────────────────────────── */

/// Options de mise en forme.
#[derive(Debug, Clone)]
pub struct PrettyOptions {
    /// Active les couleurs ANSI (stdout interactif conseillé).
    pub color: bool,
    /// Longueur max des chaînes (prévisualisation).
    pub max_str: usize,
    /// Longueur max des bytes (prévisualisation hex).
    pub max_bytes: usize,
    /// Utiliser l’hex **majuscule** pour les bytes.
    pub hex_upper: bool,
    /// Afficher le type des constantes dans les tableaux.
    pub show_types: bool,
    /// Afficher les numéros de ligne dans le listing.
    pub show_line_numbers: bool,
    /// Afficher le PC et l’adresse de destination (pour jumps).
    pub show_pc: bool,
}

impl Default for PrettyOptions {
    fn default() -> Self {
        Self {
            color: true,
            max_str: 80,
            max_bytes: 32,
            hex_upper: false,
            show_types: true,
            show_line_numbers: true,
            show_pc: true,
        }
    }
}

/// Couleurs de base (sobres).
#[derive(Clone, Copy)]
pub enum Col { Gray, Red, Green, Yellow, Blue, Magenta, Cyan, White }

fn ansi_code(c: Col) -> u8 {
    match c {
        Col::Gray => 90, Col::Red => 31, Col::Green => 32, Col::Yellow => 33,
        Col::Blue => 34, Col::Magenta => 35, Col::Cyan => 36, Col::White => 97,
    }
}

/// Emballe une string avec une couleur (si activée).
pub fn paint(s: &str, col: Col, bold: bool, opts: &PrettyOptions) -> String {
    if !opts.color { return s.into(); }
    let mut out = String::new();
    let _ = write!(
        out,
        "\x1b[{};{}m{}\x1b[0m",
        if bold {1} else {0},
        ansi_code(col),
        s
    );
    out
}
pub fn dim(s: &str, opts: &PrettyOptions) -> String {
    if !opts.color { return s.into(); }
    format!("\x1b[2m{}\x1b[0m", s)
}
pub fn bold(s: &str, opts: &PrettyOptions) -> String {
    if !opts.color { return s.into(); }
    format!("\x1b[1m{}\x1b[0m", s)
}

/* ───────────────────────────── Préviz & échappes ───────────────────────────── */

/// Échappe une chaîne pour affichage (\" \\ \n \t \r et contrôles).
pub fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => { let _ = write!(out, "\\x{:02X}", c as u32); }
            c => out.push(c),
        }
    }
    out
}

/// Tronque avec une ellipse si nécessaire.
pub fn shorten(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max]) }
}

/// Prévisualisation de chaîne (échappée + tronquée).
pub fn preview_str(s: &str, max_len: usize) -> String {
    let esc = escape_str(s);
    let cut = shorten(&esc, max_len);
    format!("\"{}\"", cut)
}

/// Prévisualisation d’un buffer bytes en hex (tronqué).
pub fn preview_bytes(bytes: &[u8], max_len: usize, upper: bool) -> String {
    let mut out = String::new();
    let show = bytes.len().min(max_len);
    for (i, b) in bytes.iter().take(show).enumerate() {
        if i > 0 { out.push(' '); }
        if upper { let _ = write!(out, "{:02X}", b); }
        else     { let _ = write!(out, "{:02x}", b); }
    }
    if bytes.len() > show { let _ = write!(out, " … (len={})", bytes.len()); }
    out
}

/// Hexdump simple (16 colonnes).
pub fn hexdump(bytes: &[u8], limit: usize) -> String {
    let mut s = String::new();
    let end = bytes.len().min(limit);
    let _ = writeln!(s, "# Hexdump (len={}, limit={})", bytes.len(), if limit==usize::MAX {"∞".into()} else {limit.to_string()});
    let mut i = 0usize;
    while i < end {
        let line = &bytes[i..end.min(i + 16)];
        let _ = write!(s, "{:08x}  ", i);
        for j in 0..16 {
            if j < line.len() { let _ = write!(s, "{:02x} ", line[j]); } else { let _ = write!(s, "   "); }
            if j == 7 { let _ = write!(s, " "); }
        }
        let _ = write!(s, " |");
        for &b in line {
            let c = if (32..=126).contains(&b) { b as char } else { '.' };
            let _ = write!(s, "{c}");
        }
        let _ = writeln!(s, "|");
        i += 16;
    }
    s
}

/* ───────────────────────────── Valeurs & constantes ───────────────────────────── */

/// Mise en forme d’un `ConstValue` (courte).
pub fn pretty_const(c: &ConstValue, opts: &PrettyOptions) -> String {
    use ConstValue::*;
    match c {
        Null      => paint("null", Col::Gray, false, opts),
        Bool(true)=> paint("true", Col::Green, true, opts),
        Bool(false)=> paint("false", Col::Red, false, opts),
        I64(i)    => paint(&i.to_string(), Col::Cyan, false, opts),
        F64(x)    => paint(&format_float(*x), Col::Cyan, false, opts),
        Str(s)    => paint(&preview_str(s, opts.max_str), Col::Yellow, false, opts),
        Bytes(b)  => {
            let body = preview_bytes(b, opts.max_bytes, opts.hex_upper);
            format!("{}[{}] {{ {body} }}", paint("bytes", Col::Magenta, true, opts), b.len())
        }
    }
}

/// Alias explicite (identique pour l’instant).
pub fn pretty_value(v: &ConstValue, opts: &PrettyOptions) -> String {
    pretty_const(v, opts)
}

fn format_float(x: f64) -> String {
    // formatte proprement 1.0 comme "1" ? On garde 1.0 pour éviter la confusion.
    let s = format!("{:?}", x); // stable, sans “scientifique” forcé
    s
}

/* ───────────────────────────── Tables & colonnes ───────────────────────────── */

/// Construit une **ligne** tabulaire à colonnes espacées (séparateur = 2 espaces).
pub fn tabrow(cols: &[&str]) -> String {
    // Calcul d’écarts minimaliste : on ne connaît pas les largeurs globales ici.
    // Utilisé principalement avec `tabulate_*` qui pré-calculent les largeurs.
    cols.join("  ")
}

/// Séparateur de tableau à partir de largeurs.
pub fn tabsep(widths: &[usize], ch: char) -> String {
    let mut s = String::new();
    for (i, w) in widths.iter().enumerate() {
        if i>0 { s.push_str("  "); }
        for _ in 0..*w { s.push(ch); }
    }
    s
}

/* ───────────────────────────── Chunk : header & tableaux ───────────────────────────── */

/// En-tête compact d’un chunk.
pub fn pretty_chunk_header(chunk: &Chunk, title: &str, opts: &PrettyOptions) -> String {
    let mut s = String::new();
    let head = format!("== {title} ==");
    let head = bold(&head, opts);
    let _ = writeln!(s, "{head}");
    let _ = writeln!(
        s,
        "• version: {}   stripped: {}   ops: {}   consts: {}   hash: {}",
        chunk.version(),
        paint(if chunk.flags().stripped {"true"} else {"false"}, Col::Magenta, false, opts),
        paint(&chunk.ops.len().to_string(), Col::Blue, false, opts),
        paint(&chunk.consts.len().to_string(), Col::Blue, false, opts),
        paint(&format!("0x{:016x}", chunk.compute_hash()), Col::Yellow, false, opts),
    );
    s
}

/// Tableau du pool de constantes (type + aperçu).
pub fn pretty_const_pool_table(chunk: &Chunk, opts: &PrettyOptions) -> String {
    let mut s = String::new();
    let title = bold("# Const Pool", opts);
    let _ = writeln!(s, "{title}");

    if chunk.consts.len() == 0 {
        let _ = writeln!(s, "  {}", dim("<vide>", opts));
        return s;
    }

    // Calcule largeurs (index/type/preview)
    let mut w_ix = 3usize;
    let mut w_ty = 4usize;
    let mut rows = Vec::<(String, String, String)>::new();

    for (ix, c) in chunk.consts.iter() {
        let ty = const_type(c);
        let prev = const_preview(c, opts);
        let a = format!("{ix:03}");
        w_ix = max(w_ix, a.len());
        w_ty = max(w_ty, ty.len());
        rows.push((a, ty.to_string(), prev));
    }

    let header = format!("{:>ixw$}  {:tyw$}  {}", "IDX", "TYPE", "PREVIEW",
        ixw=w_ix, tyw=w_ty);
    let _ = writeln!(s, "  {}", paint(&header, Col::White, true, opts));
    let _ = writeln!(s, "  {}", tabsep(&[w_ix, w_ty, 40], '─')); // 40 = min visuel

    for (a, ty, prev) in rows {
        let _ = writeln!(s, "  {:>ixw$}  {:tyw$}  {}", a, ty, prev, ixw=w_ix, tyw=w_ty);
    }

    s
}

/// Tableau des lignes (plages PC→line).
pub fn pretty_line_table(chunk: &Chunk, opts: &PrettyOptions) -> String {
    let mut s = String::new();
    let title = bold("# Line Table (PC ranges)", opts);
    let _ = writeln!(s, "{title}");
    let mut any = false;
    for (r, line) in chunk.lines.iter_ranges() {
        any = true;
        let _ = writeln!(s, "  [{:05}..{:05})  line {}", r.start, r.end, line);
    }
    if !any { let _ = writeln!(s, "  {}", dim("<aucune info de ligne>", opts)); }
    s
}

/// Bloc debug (fichiers, symbols).
pub fn pretty_debug_info(chunk: &Chunk, opts: &PrettyOptions) -> String {
    let mut s = String::new();
    let title = bold("# Debug", opts);
    let _ = writeln!(s, "{title}");

    match &chunk.debug.main_file {
        Some(m) => { let _ = writeln!(s, "  main_file: {}", paint(m, Col::Yellow, false, opts)); }
        None => { let _ = writeln!(s, "  main_file: {}", dim("(none)", opts)); }
    }
    if !chunk.debug.files.is_empty() {
        let _ = writeln!(s, "  files ({}):", chunk.debug.files.len());
        for f in &chunk.debug.files { let _ = writeln!(s, "    - {}", f); }
    } else {
        let _ = writeln!(s, "  files: {}", dim("(none)", opts));
    }
    if !chunk.debug.symbols.is_empty() {
        let _ = writeln!(s, "  symbols ({}):", chunk.debug.symbols.len());
        for (sym, pc) in &chunk.debug.symbols {
            let _ = writeln!(s, "    - {:05}  {}", pc, paint(sym, Col::Cyan, false, opts));
        }
    } else {
        let _ = writeln!(s, "  symbols: {}", dim("(none)", opts));
    }

    s
}

/* ───────────────────────────── Listing d’opcodes ───────────────────────────── */

/// Joli listing du code (avec labels de cibles).
pub fn pretty_code_listing(chunk: &Chunk, opts: &PrettyOptions) -> String {
    let mut s = String::new();
    let title = bold("# Code", opts);
    let _ = writeln!(s, "{title}");

    let labels = compute_labels(chunk);

    for (pc_usize, op) in chunk.ops.iter().enumerate() {
        let pc = pc_usize as u32;

        if let Some(lbl) = labels.get(&pc) {
            let _ = writeln!(s, "{}", paint(lbl, Col::Magenta, true, opts));
        }

        let line = chunk.lines.line_for_pc(pc);

        let row = pretty_op_line(chunk, pc, op, line, &labels, opts);
        let _ = writeln!(s, "  {}", row);
    }

    s
}

/// Une **ligne** pour un opcode, avec annotations.
pub fn pretty_op_line(
    chunk: &Chunk,
    pc: u32,
    op: &Op,
    line: Option<u32>,
    labels: &std::collections::HashMap<u32, String>,
    opts: &PrettyOptions,
) -> String {
    use Op::*;
    let mut left = String::new();

    if opts.show_pc {
        let _ = write!(left, "{:05}", pc);
    }
    if opts.show_line_numbers {
        let _ = write!(left, " (line {:>4})", line.map(|l| l.to_string()).unwrap_or_else(|| "-".into()));
    }

    // Corps de l’instruction + annexes
    let main = match *op {
        LoadConst(ix) => match chunk.consts.get(ix) {
            Some(ConstValue::Str(ref s)) => format!("LoadConst {}   ; {}", ix, preview_str(s, opts.max_str)),
            Some(ref v) => format!("LoadConst {}   ; {}", ix, pretty_const(v, opts)),
            None => format!("LoadConst {}   ; {}", ix, paint("<invalid>", Col::Red, false, opts)),
        },
        LoadLocal(ix)    => format!("LoadLocal {}", ix),
        StoreLocal(ix)   => format!("StoreLocal {}", ix),
        LoadUpvalue(ix)  => format!("LoadUpvalue {}", ix),
        StoreUpvalue(ix) => format!("StoreUpvalue {}", ix),
        MakeClosure(fi,n)=> format!("MakeClosure func={} upvalues={}", fi, n),
        Call(argc)       => format!("Call argc={}", argc),
        TailCall(argc)   => format!("TailCall argc={}", argc),
        Jump(off)        => pretty_jump("Jump", pc, off, labels, opts),
        JumpIfFalse(off) => pretty_jump("JumpIfFalse", pc, off, labels, opts),
        Return           => "Return".into(),
        ReturnVoid       => "ReturnVoid".into(),
        Nop              => "Nop".into(),
        Print            => "Print".into(),
        Add              => "Add".into(),
        Sub              => "Sub".into(),
        Mul              => "Mul".into(),
        Div              => "Div".into(),
        Mod              => "Mod".into(),
        Neg              => "Neg".into(),
        Not              => "Not".into(),
        Eq               => "Eq".into(),
        Ne               => "Ne".into(),
        Lt               => "Lt".into(),
        Le               => "Le".into(),
        Gt               => "Gt".into(),
        Ge               => "Ge".into(),
        LoadTrue         => "LoadTrue".into(),
        LoadFalse        => "LoadFalse".into(),
        LoadNull         => "LoadNull".into(),
        Pop              => "Pop".into(),
    };

    format!("{:<24}  {}", dim(&left, opts), main)
}

fn pretty_jump(
    name: &str,
    pc: u32,
    off: i32,
    labels: &std::collections::HashMap<u32, String>,
    opts: &PrettyOptions,
) -> String {
    let dest = (pc as i64 + 1 + off as i64).max(0) as u32;
    let target = labels.get(&dest).cloned().unwrap_or_else(|| dest.to_string());
    format!("{} {:+}  -> {}", name, off, paint(&target, Col::Blue, false, opts))
}

/// Calcule labels `L0001..` pour toutes les destinations de saut.
pub fn compute_labels(chunk: &Chunk) -> std::collections::HashMap<u32, String> {
    use std::collections::HashMap;
    let mut set = HashMap::<u32, String>::new();
    let mut targets = Vec::<u32>::new();
    for (pc_usize, op) in chunk.ops.iter().enumerate() {
        let pc = pc_usize as u32;
        match *op {
            Op::Jump(ofs) | Op::JumpIfFalse(ofs) => {
                let dest = pc as i64 + 1 + ofs as i64;
                if dest >= 0 && (dest as usize) < chunk.ops.len() {
                    targets.push(dest as u32);
                }
            }
            _ => {}
        }
    }
    targets.sort_unstable();
    targets.dedup();
    for (i, pc) in targets.into_iter().enumerate() {
        set.insert(pc, format!("L{:04}", i + 1));
    }
    set
}

/* ───────────────────────────── Rapport complet ───────────────────────────── */

/// Rapport complet lisible humain (header + consts + lines + debug + code).
pub fn pretty_chunk_report(chunk: &Chunk, title: &str, opts: &PrettyOptions) -> String {
    let mut s = String::new();
    s.push_str(&pretty_chunk_header(chunk, title, opts));
    s.push('\n');
    s.push_str(&pretty_const_pool_table(chunk, opts));
    s.push('\n');
    s.push_str(&pretty_line_table(chunk, opts));
    s.push('\n');
    s.push_str(&pretty_debug_info(chunk, opts));
    s.push('\n');
    s.push_str(&pretty_code_listing(chunk, opts));
    s
}

/* ───────────────────────────── Helpers internes ───────────────────────────── */

fn const_type(c: &ConstValue) -> &'static str {
    use ConstValue::*;
    match c {
        Null => "null",
        Bool(_) => "bool",
        I64(_) => "i64",
        F64(_) => "f64",
        Str(_) => "str",
        Bytes(_) => "bytes",
    }
}

fn const_preview(c: &ConstValue, opts: &PrettyOptions) -> String {
    use ConstValue::*;
    match c {
        Str(s) => preview_str(s, opts.max_str),
        Bytes(b) => format!("{{ {} }}", preview_bytes(b, opts.max_bytes, opts.hex_upper)),
        other => pretty_const(other, opts),
    }
}

/* ───────────────────────────── Tests “fumants” ───────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::chunk::{Chunk, ChunkFlags};

    #[test]
    fn color_toggles() {
        let opts = PrettyOptions { color: false, ..Default::default() };
        assert_eq!(paint("x", Col::Red, true, &opts), "x".to_string());
        let opts = PrettyOptions { color: true, ..Default::default() };
        assert!(paint("x", Col::Red, true, &opts).starts_with("\x1b[1;31m"));
    }

    #[test]
    fn const_preview_str_bytes() {
        let o = PrettyOptions::default();
        let a = pretty_const(&ConstValue::Str("hé\n".into()), &o);
        assert!(a.contains("\\n"));
        let b = pretty_const(&ConstValue::Bytes(vec![0xDE,0xAD,0xBE,0xEF]), &o);
        assert!(b.contains("bytes"));
    }

    #[test]
    fn report_smoke() {
        let c = Chunk::new(ChunkFlags { stripped: false });
        let opts = PrettyOptions::default();
        let rep = pretty_chunk_report(&c, "demo", &opts);
        assert!(rep.contains("== demo =="));
        assert!(rep.contains("# Const Pool"));
        assert!(rep.contains("# Code"));
    }
}
