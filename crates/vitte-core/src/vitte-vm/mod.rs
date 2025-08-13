//! vm/mod.rs — Mini VM de test pour exécuter un sous-ensemble d'opcodes.
//! Ce n’est PAS la VM finale, juste de quoi valider les chunks.

use crate::bytecode::{Chunk, ConstValue, Op};
use anyhow::{bail, Result};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
}

#[derive(Debug, Error)]
pub enum VmError {
    #[error("stack underflow")]
    StackUnderflow,
    #[error("type error: {0}")]
    Type(String),
    #[error("bad const index {0}")]
    BadConst(u32),
    #[error("pc out of range: {0}")]
    PcOob(usize),
}

pub struct Vm {
    stack: Vec<Value>,
    locals: Vec<Value>,
}

impl Vm {
    pub fn new() -> Self {
        Self { stack: Vec::with_capacity(256), locals: vec![Value::Null; 256] }
    }

    fn push(&mut self, v: Value) { self.stack.push(v) }
    fn pop(&mut self) -> Result<Value> { self.stack.pop().ok_or_else(|| VmError::StackUnderflow.into()) }

    pub fn run(&mut self, chunk: &Chunk) -> Result<()> {
        let mut pc: isize = 0;
        let ops = &chunk.ops;

        while (pc as usize) < ops.len() {
            let op = *ops.get(pc as usize).ok_or(VmError::PcOob(pc as usize))?;
            pc += 1;

            use Op::*;
            match op {
                Nop => {}

                LoadConst(ix) => {
                    let c = chunk.consts.get(ix).ok_or(VmError::BadConst(ix))?;
                    self.push(match c {
                        ConstValue::Null => Value::Null,
                        ConstValue::Bool(b) => Value::Bool(*b),
                        ConstValue::I64(i) => Value::I64(*i),
                        ConstValue::F64(x) => Value::F64(*x),
                        ConstValue::Str(s) => Value::Str(s.clone()),
                        ConstValue::Bytes(_) => bail!("Bytes const not supported in mini-VM"),
                    });
                }
                LoadTrue => self.push(Value::Bool(true)),
                LoadFalse => self.push(Value::Bool(false)),
                LoadNull => self.push(Value::Null),

                LoadLocal(ix) => {
                    let v = self.locals.get(ix as usize).cloned().unwrap_or(Value::Null);
                    self.push(v);
                }
                StoreLocal(ix) => {
                    let v = self.pop()?;
                    if (ix as usize) >= self.locals.len() { self.locals.resize(ix as usize + 1, Value::Null); }
                    self.locals[ix as usize] = v;
                }

                Pop => { self.pop()?; }

                // ----- Arith -----
                Add => bin_num(self, |a,b| a+b)?,
                Sub => bin_num(self, |a,b| a-b)?,
                Mul => bin_num(self, |a,b| a*b)?,
                Div => bin_num(self, |a,b| a/b)?,
                Mod => bin_num(self, |a,b| a % b)?,
                Neg => {
                    let v = self.pop()?;
                    match v {
                        Value::I64(i) => self.push(Value::I64(-i)),
                        Value::F64(x) => self.push(Value::F64(-x)),
                        _ => bail!(VmError::Type("Neg expects number".into())),
                    }
                }
                Not => {
                    let v = self.pop()?;
                    match v {
                        Value::Bool(b) => self.push(Value::Bool(!b)),
                        Value::Null => self.push(Value::Bool(true)),
                        _ => self.push(Value::Bool(false)),
                    }
                }

                // ----- Comparaisons -----
                Eq => cmp_eq(self)?,
                Ne => { cmp_eq(self)?; flip_bool(self)?; }
                Lt => bin_cmp(self, |a,b| a<b)?,
                Le => bin_cmp(self, |a,b| a<=b)?,
                Gt => bin_cmp(self, |a,b| a>b)?,
                Ge => bin_cmp(self, |a,b| a>=b)?,

                // ----- Branch -----
                Jump(off) => { pc += off as isize; }
                JumpIfFalse(off) => {
                    let v = self.pop()?;
                    let cond = match v {
                        Value::Bool(b) => b,
                        Value::Null => false,
                        _ => true,
                    };
                    if !cond { pc += off as isize; }
                }

                // ----- Appels : no-op MVP -----
                Call(_argc) => bail!("Call not implemented in mini-VM"),
                TailCall(_argc) => bail!("TailCall not implemented in mini-VM"),

                // ----- Retour -----
                Return | ReturnVoid => break,

                // ----- Debug / I/O -----
                Print => {
                    let v = self.pop()?;
                    println!("{v:?}");
                }

                // Fermetures : non impl.
                MakeClosure(_, _) | LoadUpvalue(_) | StoreUpvalue(_) => {
                    bail!("Closures not implemented in mini-VM")
                }
            }
        }
        Ok(())
    }
}

fn as_num(v: Value) -> Result<f64> {
    Ok(match v {
        Value::I64(i) => i as f64,
        Value::F64(x) => x,
        _ => return Err(VmError::Type("number expected".into()).into()),
    })
}

fn bin_num(vm: &mut Vm, f: impl FnOnce(f64, f64) -> f64) -> Result<()> {
    let b = vm.pop()?;
    let a = vm.pop()?;
    let res = f(as_num(a)?, as_num(b)?);
    // heuristique : entier si près d’un entier
    if res.fract().abs() < 1e-12 {
        vm.push(Value::I64(res as i64));
    } else {
        vm.push(Value::F64(res));
    }
    Ok(())
}

fn bin_cmp(vm: &mut Vm, f: impl FnOnce(f64, f64) -> bool) -> Result<()> {
    let b = vm.pop()?;
    let a = vm.pop()?;
    let res = f(as_num(a)?, as_num(b)?);
    vm.push(Value::Bool(res));
    Ok(())
}

fn cmp_eq(vm: &mut Vm) -> Result<()> {
    let b = vm.pop()?;
    let a = vm.pop()?;
    let res = match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::I64(x), Value::I64(y)) => x == y,
        (Value::F64(x), Value::F64(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        _ => false,
    };
    vm.push(Value::Bool(res));
    Ok(())
}

fn flip_bool(vm: &mut Vm) -> Result<()> {
    let v = vm.pop()?;
    match v {
        Value::Bool(b) => vm.push(Value::Bool(!b)),
        _ => vm.push(Value::Bool(false)),
    }
    Ok(())
}
