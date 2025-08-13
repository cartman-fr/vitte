//! vitte-core/src/runtime/eval.rs
//!
//! Évaluateur “léger” de bytecode Vitte pour tests/REPL internes.
//! Objectif : exécuter un `Chunk` sans dépendre de la VM complète.
//!
//! Gère (MVP) :
//!   - Constantes: Null/Bool/I64/F64/Str
//!   - Pile : push/pop
//!   - Arith: Add/Sub/Mul/Div/Mod, Neg, Not
//!   - Comparaisons: Eq, Ne, Lt, Le, Gt, Ge
//!   - Contrôle: Jump, JumpIfFalse, Pop, Return/ReturnVoid, Nop
//!   - I/O: Print (redirigé vers un buffer capturé)
//!
//! ⚠️ Non géré (panic contrôlé) : closures, upvalues, call/tail-call (MVP).
//!
//! API:
//!   - `eval_chunk(&Chunk, EvalOptions) -> Result<EvalOutput>`
//!   - `EvalOptions { capture_stdout: bool, max_steps: Option<usize> }`
//!   - `EvalOutput { stdout: String, steps: usize }`

use std::fmt;
use anyhow::{bail, Result};
use crate::bytecode::{Chunk, ConstValue, Op};

#[derive(Debug, Clone)]
pub struct EvalOptions {
    /// Capture le `Print` dans un buffer; sinon, écrit sur stdout réel.
    pub capture_stdout: bool,
    /// Garde-fou: limite d’instructions pour éviter les boucles infinies.
    pub max_steps: Option<usize>,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self { capture_stdout: true, max_steps: Some(1_000_000) }
    }
}

#[derive(Debug, Default)]
pub struct EvalOutput {
    pub stdout: String,
    pub steps: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::I64(i)  => write!(f, "{i}"),
            Value::F64(x)  => write!(f, "{x}"),
            Value::Str(s)  => write!(f, "{s}"),
        }
    }
}

/// Exécute un `Chunk` de bytecode avec un mini-interpréteur.
/// Idéal pour tests, REPL, et validations rapides.
pub fn eval_chunk(chunk: &Chunk, opts: EvalOptions) -> Result<EvalOutput> {
    let mut ev = Evaluator::new(opts);
    ev.run(chunk)?;
    Ok(EvalOutput { stdout: ev.stdout, steps: ev.steps })
}

struct Evaluator {
    stack: Vec<Value>,
    stdout: String,
    steps: usize,
    opts: EvalOptions,
}

impl Evaluator {
    fn new(opts: EvalOptions) -> Self {
        Self {
            stack: Vec::with_capacity(256),
            stdout: String::new(),
            steps: 0,
            opts,
        }
    }

    fn run(&mut self, chunk: &Chunk) -> Result<()> {
        let ops = &chunk.ops;
        let mut pc: isize = 0;

        while (pc as usize) < ops.len() {
            // garde-fou anti-boucle
            self.steps += 1;
            if let Some(limit) = self.opts.max_steps {
                if self.steps > limit {
                    bail!("limite d’instructions atteinte ({limit})");
                }
            }

            let op = *ops.get(pc as usize).ok_or_else(|| anyhow::anyhow!("pc hors limites"))?;
            pc += 1;

            use Op::*;
            match op {
                // ---- Structure
                Nop => {}
                Return | ReturnVoid => break,

                // ---- Constantes
                LoadTrue  => self.push(Value::Bool(true)),
                LoadFalse => self.push(Value::Bool(false)),
                LoadNull  => self.push(Value::Null),
                LoadConst(ix) => {
                    let c = chunk.consts.get(ix).ok_or_else(|| anyhow::anyhow!("const index invalide {ix}"))?;
                    self.push(match c {
                        ConstValue::Null      => Value::Null,
                        ConstValue::Bool(b)   => Value::Bool(*b),
                        ConstValue::I64(i)    => Value::I64(*i),
                        ConstValue::F64(x)    => Value::F64(*x),
                        ConstValue::Str(s)    => Value::Str(s.clone()),
                        ConstValue::Bytes(_)  => bail!("Const Bytes non supportée par l’évaluateur MVP"),
                    });
                }

                // ---- Variables locales (non supportées ici)
                LoadLocal(_) | StoreLocal(_) => bail!("Locals non supportés par l’évaluateur MVP"),

                // ---- Stack
                Pop => { let _ = self.pop()?; }

                // ---- Arith
                Add => self.bin_num(|a,b| a + b)?,
                Sub => self.bin_num(|a,b| a - b)?,
                Mul => self.bin_num(|a,b| a * b)?,
                Div => self.bin_num(|a,b| a / b)?,
                Mod => self.bin_int(|a,b| a % b)?,
                Neg => {
                    let v = self.pop()?;
                    match v {
                        Value::I64(i) => self.push(Value::I64(-i)),
                        Value::F64(x) => self.push(Value::F64(-x)),
                        _ => bail!("Neg attend un nombre"),
                    }
                }
                Not => {
                    let v = self.pop()?;
                    let b = match v {
                        Value::Bool(b) => !b,
                        Value::Null => true,
                        _ => false,
                    };
                    self.push(Value::Bool(b));
                }

                // ---- Comparaisons
                Eq => self.cmp_eq()?,
                Ne => { self.cmp_eq()?; self.flip_bool()?; }
                Lt => self.bin_cmp(|a,b| a <  b)?,
                Le => self.bin_cmp(|a,b| a <= b)?,
                Gt => self.bin_cmp(|a,b| a >  b)?,
                Ge => self.bin_cmp(|a,b| a >= b)?,

                // ---- Contrôle
                Jump(off) => { pc += off as isize; }
                JumpIfFalse(off) => {
                    let cond = self.truthy()?;
                    if !cond { pc += off as isize; }
                }

                // ---- Appels (non supportés MVP)
                Call(_) | TailCall(_) => bail!("Call/TailCall non supportés par l’évaluateur MVP"),

                // ---- I/O
                Print => {
                    let v = self.pop()?;
                    if self.opts.capture_stdout {
                        use std::fmt::Write;
                        let _ = writeln!(&mut self.stdout, "{v}");
                    } else {
                        println!("{v}");
                    }
                }

                // ---- Fermetures & upvalues (non supportés)
                MakeClosure(_, _) | LoadUpvalue(_) | StoreUpvalue(_) => {
                    bail!("Closures/upvalues non supportés par l’évaluateur MVP")
                }
            }
        }

        Ok(())
    }

    // ---------- Helpers VM-like ----------

    fn push(&mut self, v: Value) { self.stack.push(v) }

    fn pop(&mut self) -> Result<Value> {
        self.stack.pop().ok_or_else(|| anyhow::anyhow!("stack underflow"))
    }

    fn as_num(v: Value) -> Result<f64> {
        Ok(match v {
            Value::I64(i) => i as f64,
            Value::F64(x) => x,
            _ => bail!("nombre attendu"),
        })
    }

    fn as_int(v: Value) -> Result<i64> {
        Ok(match v {
            Value::I64(i) => i,
            _ => bail!("entier attendu"),
        })
    }

    fn bin_num(&mut self, f: impl FnOnce(f64, f64) -> f64) -> Result<()> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = f(Self::as_num(a)?, Self::as_num(b)?);
        if r.fract().abs() < 1e-12 {
            self.push(Value::I64(r as i64));
        } else {
            self.push(Value::F64(r));
        }
        Ok(())
    }

    fn bin_int(&mut self, f: impl FnOnce(i64, i64) -> i64) -> Result<()> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = f(Self::as_int(a)?, Self::as_int(b)?);
        self.push(Value::I64(r));
        Ok(())
    }

    fn bin_cmp(&mut self, f: impl FnOnce(f64, f64) -> bool) -> Result<()> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = f(Self::as_num(a)?, Self::as_num(b)?);
        self.push(Value::Bool(r));
        Ok(())
    }

    fn cmp_eq(&mut self) -> Result<()> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::I64(x), Value::I64(y)) => x == y,
            (Value::F64(x), Value::F64(y)) => x == y,
            (Value::Str(x), Value::Str(y)) => x == y,
            _ => false,
        };
        self.push(Value::Bool(r));
        Ok(())
    }

    fn flip_bool(&mut self) -> Result<()> {
        let v = self.pop()?;
        match v {
            Value::Bool(b) => self.push(Value::Bool(!b)),
            _ => self.push(Value::Bool(false)),
        }
        Ok(())
    }

    fn truthy(&mut self) -> Result<bool> {
        let v = self.pop()?;
        Ok(match v {
            Value::Null => false,
            Value::Bool(b) => b,
            _ => true,
        })
    }
}
