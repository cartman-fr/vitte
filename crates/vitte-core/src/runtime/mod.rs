//! runtime/mod.rs — Façade runtime de Vitte
//!
//! Regroupe et réexporte :
//! - [`eval`]   : VM pile (exécution du bytecode, appels host, limites, trace)
//! - [`parser`] : parseur de littéraux runtime (null/bool/ints/float/str/bytes/hex)
//!
//! Fournit aussi des **helpers** de haut niveau (`run*`) et un **host standard**
//! minimal (`StdHost`) pour des appels `Call` basiques (ex: `"io.println"`).
//!
//! Exemples rapides :
//! ```no_run
//! use vitte_core::bytecode::chunk::{Chunk, ChunkFlags, ConstValue};
//! use vitte_core::bytecode::op::Op;
//! use vitte_core::runtime::{run, StdHost, EvalOptions};
//!
//! // Construire un mini-chunk : print "Hello" ; retv
//! let mut c = Chunk::new(ChunkFlags{ stripped:false });
//! let s = c.add_const(ConstValue::Str("Hello".into()));
//! c.push_op(Op::LoadConst(s), Some(1));
//! c.push_op(Op::Print,        Some(1));
//! c.push_op(Op::ReturnVoid,   Some(1));
//!
//! // Exécuter avec host standard (gestion de Call et I/O simple)
//! let mut opts = EvalOptions::default();
//! opts.host = Some(Box::new(StdHost::default()));
//! let outcome = run(&c, opts).unwrap();
//! assert!(outcome.halted);
//! ```

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, unused_must_use)]

/* ───────────────────────────── Sous-modules ───────────────────────────── */

pub mod eval;
pub mod parser;

/* ───────────────────────────── Réexports utiles ───────────────────────────── */

pub use eval::{
    ExecOutcome, EvalError, EvalErrorKind, EvalOptions, Host, Vm,
};
pub use parser::{parse_list, parse_value, try_number, ParseError, Pos};

/* ───────────────────────────── Helpers haut niveau ───────────────────────────── */

use crate::bytecode::chunk::{Chunk, ConstValue};
use crate::bytecode::op::Op;

/// Exécute un `Chunk` avec options (limites, trace, host…).
///
/// Raccourci : `Vm::new(chunk).with_options(opts).run()`
pub fn run(chunk: &Chunk, opts: EvalOptions) -> Result<ExecOutcome, EvalError> {
    Vm::new(chunk).with_options(opts).run()
}

/// Exécute un `Chunk` avec **options par défaut** (pas d’host).
pub fn run_default(chunk: &Chunk) -> Result<ExecOutcome, EvalError> {
    Vm::new(chunk).run()
}

/* ───────────────────────────── Host standard (optionnel) ───────────────────────────── */

/// Un **host minimal** pour `Call`/`TailCall` basé sur des noms de fonctions
/// simples (chaîne en pile). Cible : scripts, tests, REPL.
///
/// Fonctions supportées (nom du callee, arité) :
/// - `"io.print"`(1)    → imprime la valeur (sans newline), retourne `null`
/// - `"io.println"`(1)  → imprime la valeur + newline, retourne `null`
/// - `"string.len"`(1)  → `str -> i64`
/// - `"bytes.len"`(1)   → `bytes -> i64`
/// - `"math.add"`(2)    → `num,num -> num`
/// - `"math.sub"`(2)    → `num,num -> num`
/// - `"math.mul"`(2)    → `num,num -> num`
/// - `"math.div"`(2)    → `num,num -> num` (division flottante si besoin)
/// - `"debug.typeof"`(1)→ `value -> str`
///
/// NB : le jeu peut être étendu sans casser l’API (ajoute des branches).
#[derive(Default)]
pub struct StdHost;

impl eval::Host for StdHost {
    fn call(&mut self, name: &str, args: &[ConstValue]) -> Result<ConstValue, EvalError> {
        use ConstValue::*;
        let bad_arity = |exp: usize| EvalError {
            kind: EvalErrorKind::BadCall("mauvaise arité"),
            pc: 0, line: None,
        };
        let type_err = |msg: &'static str| EvalError {
            kind: EvalErrorKind::Type(msg),
            pc: 0, line: None,
        };

        match name {
            // ---- I/O ----
            "io.print" => {
                if args.len() != 1 { return Err(bad_arity(1)); }
                print!("{}", pretty_value_inline(&args[0]));
                Ok(Null)
            }
            "io.println" => {
                if args.len() != 1 { return Err(bad_arity(1)); }
                println!("{}", pretty_value_inline(&args[0]));
                Ok(Null)
            }

            // ---- String/Bytes ----
            "string.len" => {
                if args.len() != 1 { return Err(bad_arity(1)); }
                match &args[0] { Str(s) => Ok(I64(s.chars().count() as i64)), _ => Err(type_err("string.len: attendu str")) }
            }
            "bytes.len" => {
                if args.len() != 1 { return Err(bad_arity(1)); }
                match &args[0] { Bytes(b) => Ok(I64(b.len() as i64)), _ => Err(type_err("bytes.len: attendu bytes")) }
            }

            // ---- Math ----
            "math.add" => {
                if args.len() != 2 { return Err(bad_arity(2)); }
                num2(args, |a,b| Ok(a+b), |a,b| Ok(a+b))
            }
            "math.sub" => {
                if args.len() != 2 { return Err(bad_arity(2)); }
                num2(args, |a,b| Ok(a-b), |a,b| Ok(a-b))
            }
            "math.mul" => {
                if args.len() != 2 { return Err(bad_arity(2)); }
                num2(args, |a,b| Ok(a*b), |a,b| Ok(a*b))
            }
            "math.div" => {
                if args.len() != 2 { return Err(bad_arity(2)); }
                match (&args[0], &args[1]) {
                    (I64(_), I64(0)) => Err(EvalError { kind: EvalErrorKind::DivByZero, pc: 0, line: None }),
                    (I64(a), I64(b)) => Ok(F64(*a as f64 / *b as f64)),
                    (I64(a), F64(b)) => Ok(F64(*a as f64 / *b)),
                    (F64(a), I64(b)) => Ok(F64(*a / *b as f64)),
                    (F64(a), F64(b)) => Ok(F64(*a / *b)),
                    _ => Err(type_err("math.div: args non numériques")),
                }
            }

            // ---- Debug ----
            "debug.typeof" => {
                if args.len() != 1 { return Err(bad_arity(1)); }
                Ok(Str(type_name_of(&args[0]).into()))
            }

            // ---- Par défaut : inconnu ----
            _ => Err(EvalError { kind: EvalErrorKind::BadCall("callee inconnu"), pc: 0, line: None }),
        }
    }
}

/* ───────────────────────────── Utilities StdHost ───────────────────────────── */

fn pretty_value_inline(v: &ConstValue) -> String {
    use ConstValue::*;
    match v {
        Null       => "null".into(),
        Bool(b)    => b.to_string(),
        I64(i)     => i.to_string(),
        F64(x)     => format!("{x:?}"),
        Str(s)     => {
            let mut t = String::new();
            for ch in s.chars() {
                match ch {
                    '\n' => t.push_str("\\n"),
                    '\t' => t.push_str("\\t"),
                    '\r' => t.push_str("\\r"),
                    '\\' => t.push_str("\\\\"),
                    '"'  => t.push_str("\\\""),
                    c if c.is_control() => { use core::fmt::Write; let _ = write!(t, "\\x{:02X}", c as u32); }
                    c => t.push(c),
                }
            }
            format!("\"{}\"", t)
        }
        Bytes(b)   => format!("bytes[{}]", b.len()),
    }
}

fn type_name_of(v: &ConstValue) -> &'static str {
    use ConstValue::*;
    match v {
        Null => "null",
        Bool(_) => "bool",
        I64(_) => "i64",
        F64(_) => "f64",
        Str(_) => "str",
        Bytes(_) => "bytes",
    }
}

fn num2<FInt, FFloat>(args: &[ConstValue], iop: FInt, fop: FFloat) -> Result<ConstValue, EvalError>
where
    FInt: FnOnce(i64, i64) -> Result<i64, EvalError>,
    FFloat: FnOnce(f64, f64) -> Result<f64, EvalError>,
{
    use ConstValue::*;
    match (&args[0], &args[1]) {
        (I64(a), I64(b)) => Ok(I64(iop(*a, *b)?)),
        (I64(a), F64(b)) => Ok(F64(fop(*a as f64, *b)?)),
        (F64(a), I64(b)) => Ok(F64(fop(*a, *b as f64)?)),
        (F64(a), F64(b)) => Ok(F64(fop(*a, *b)?)),
        _ => Err(EvalError { kind: EvalErrorKind::Type("op num: attendu nombres"), pc: 0, line: None }),
    }
}

/* ───────────────────────────── Aides de construction simple ───────────────────────────── */

/// Construit rapidement un chunk qui **imprime** une valeur constante, pour tests.
pub fn quick_print_chunk(val: ConstValue) -> Chunk {
    use crate::bytecode::chunk::ChunkFlags;
    let mut c = Chunk::new(ChunkFlags { stripped: false });
    let ix = c.add_const(val);
    c.push_op(Op::LoadConst(ix), Some(1));
    c.push_op(Op::Print,        Some(1));
    c.push_op(Op::ReturnVoid,   Some(1));
    c
}

/* ───────────────────────────── Tests fumants ───────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::chunk::ChunkFlags;

    #[test]
    fn run_default_ok() {
        let c = quick_print_chunk(ConstValue::Str("yo".into()));
        let out = run_default(&c).unwrap();
        assert!(out.halted);
        assert_eq!(out.stack_size, 0);
    }

    #[test]
    fn stdhost_calls() {
        // callee "math.add" + 2 args → `Call(2)` → résultat sur pile → Print
        let mut c = Chunk::new(ChunkFlags { stripped: false });
        let callee = c.add_const(ConstValue::Str("math.add".into()));
        let a = c.add_const(ConstValue::I64(2));
        let b = c.add_const(ConstValue::F64(3.5));
        c.push_op(Op::LoadConst(callee), Some(1));
        c.push_op(Op::LoadConst(a), Some(1));
        c.push_op(Op::LoadConst(b), Some(1));
        c.push_op(Op::Call(2), Some(1));
        c.push_op(Op::Print, Some(1));
        c.push_op(Op::ReturnVoid, Some(1));

        let mut opts = EvalOptions::default();
        opts.host = Some(Box::new(StdHost::default()));
        let out = run(&c, opts).unwrap();
        assert!(out.halted);
    }
}
