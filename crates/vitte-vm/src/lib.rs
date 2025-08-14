//! vitte-vm — Machine virtuelle d’exécution pour le langage Vitte
//!
//! Ce crate fournit une **VM générique, sûre et extensible** pour exécuter un
//! *bytecode Vitte*. Il expose :
//!
//! - un type [`Vm`] avec configuration par [`VmOptions`],
//! - un modèle de valeurs dynamique [`Value`],
//! - un système d’erreurs riche [`VmError`],
//! - un mécanisme d’**intégration d’opcodes** via le trait [`OpAdapter`],
//! - des **fonctions natives** (host functions) et un petit *stdlib* optionnel.
//!
//! > ⚠️ **Important** : ce crate ne connaît pas vos opcodes à l’avance. Il sait
//! > boucler sur un `Chunk` et déléguer l’exécution de chaque `Op` à un
//! > *adaptateur d’opcodes*. Par défaut, tout opcode retournera `Unsupported`.
//! > Implémentez vos handlers en fournissant un `impl OpAdapter for Op` dans un
//! > module de votre projet (ou activez une feature locale si vous en avez une).
//!
//! ### Exemple d’utilisation
//!
//! ```no_run
//! use vitte_vm::{Vm, VmOptions};
//! use vitte_core::bytecode::Chunk; // Votre crate core doit exposer ce type
//!
//! # fn load_chunk() -> Chunk { unimplemented!("chargez votre bytecode") }
//! let chunk = load_chunk();
//! let mut vm = Vm::with_options(VmOptions::default().with_stdlib(true));
//! let result = vm.run(&chunk);
//! if let Err(e) = result {
//!     eprintln!("VM error: {e}");
//! }
//! ```
//!
//! ### Adapter vos opcodes
//!
//! Implémentez le trait [`OpAdapter`] pour votre type `Op` (celui de
//! `vitte_core::bytecode::Op`). Exemple minimal :
//!
//! ```ignore
//! use vitte_vm::{OpAdapter, Vm, VmResult};
//! use vitte_core::bytecode::{Chunk, Op};
//!
//! impl OpAdapter for Op {
//!     fn step(&self, vm: &mut Vm, _chunk: &Chunk) -> VmResult<()> {
//!         match self {
//!             Op::Nop => Ok(()),
//!             // … vos autres handlers …
//!             _ => Err(vitte_vm::VmError::Unsupported(format!("{self:?}"))),
//!         }
//!     }
//! }
//! ```
//!
//! Ce design **évite le couplage** fort entre la VM et le format exact de vos
//! opcodes, facilite l’évolution, et permet plusieurs backends d’instructions.

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]
#![deny(unused_must_use)]
#![warn(missing_docs)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display};
use std::rc::Rc;
use std::time::{Duration, Instant};

// Dépendances sur le "core" du langage : Chunk & Op doivent exister côté vitte_core.
// On ne suppose rien d’autre (pool de constantes, etc.).
use vitte_core::bytecode::{Chunk, Op};

/// Résultat standard de la VM.
pub type VmResult<T> = Result<T, VmError>;

/// Petit alias de GC coopératif basé sur `Rc<RefCell<T>>`.
pub type Gc<T> = Rc<RefCell<T>>;

/// Fonction native (host) : reçoit une VM et des arguments, renvoie un `Value`.
pub type NativeFn = fn(&mut Vm, &[Value]) -> VmResult<Value>;

/// Options de construction / exécution de la VM.
#[derive(Debug, Clone)]
pub struct VmOptions {
    /// Taille maximale de pile (valeurs). `None` = illimitée.
    pub stack_limit: Option<usize>,
    /// Profondeur maximale d’appels. `None` = illimitée.
    pub call_stack_limit: Option<usize>,
    /// Limite de *gas* (nombre d’étapes/opcodes) pour prévenir les boucles infinies.
    /// `None` = pas de limite.
    pub gas_limit: Option<u64>,
    /// Active le *tracing* basique (impression de chaque opcode).
    pub trace: bool,
    /// Expose un petit *stdlib* (print, clock…).
    pub stdlib: bool,
}

impl Default for VmOptions {
    fn default() -> Self {
        Self {
            stack_limit: Some(1 << 20),     // ~1M valeurs
            call_stack_limit: Some(1 << 16), // ~65k frames
            gas_limit: None,
            trace: false,
            stdlib: false,
        }
    }
}

impl VmOptions {
    /// Active/désactive le *trace*.
    pub fn with_trace(mut self, on: bool) -> Self { self.trace = on; self }
    /// Active/désactive le petit *stdlib*.
    pub fn with_stdlib(mut self, on: bool) -> Self { self.stdlib = on; self }
    /// Définit une limite de gas (étapes).
    pub fn with_gas_limit(mut self, gas: Option<u64>) -> Self { self.gas_limit = gas; self }
    /// Définit une limite de pile.
    pub fn with_stack_limit(mut self, lim: Option<usize>) -> Self { self.stack_limit = lim; self }
    /// Définit une limite de frames d’appel.
    pub fn with_call_stack_limit(mut self, lim: Option<usize>) -> Self { self.call_stack_limit = lim; self }
}

/// Valeur dynamique de la VM.
#[derive(Clone)]
pub enum Value {
    /// Unité (équivalent de `()`/`nil`).
    Unit,
    /// Booléen.
    Bool(bool),
    /// Entier signé 64-bit.
    Int(i64),
    /// Nombre flottant 64-bit.
    Float(f64),
    /// Chaîne GC.
    Str(Gc<String>),
    /// Tableau GC.
    Array(Gc<Vec<Value>>),
    /// Dictionnaire GC.
    Map(Gc<HashMap<String, Value>>),
    /// Référence à une fonction par index (côté bytecode).
    Function(FuncRef),
    /// Fermeture : fonction + *upvalues* capturées.
    Closure(Closure),
    /// Fonction native (host).
    Native(NativeFn),
}

impl Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unit => write!(f, "Unit"),
            Value::Bool(b) => write!(f, "Bool({b})"),
            Value::Int(i) => write!(f, "Int({i})"),
            Value::Float(x) => write!(f, "Float({x})"),
            Value::Str(s) => write!(f, "Str(\"{}\")", s.borrow()),
            Value::Array(a) => write!(f, "Array(len={})", a.borrow().len()),
            Value::Map(m) => write!(f, "Map(len={})", m.borrow().len()),
            Value::Function(fr) => write!(f, "Function({:?})", fr),
            Value::Closure(c) => write!(f, "Closure(fun={:?}, upvalues={})", c.func, c.upvalues.len()),
            Value::Native(_) => write!(f, "Native(<fn>)"),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unit => write!(f, "()"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::Str(s) => write!(f, "{}", s.borrow()),
            Value::Array(a) => {
                let a = a.borrow();
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Map(m) => {
                let m = m.borrow();
                write!(f, "{{")?;
                let mut first = true;
                for (k, v) in m.iter() {
                    if !first { write!(f, ", ")?; } else { first = false; }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            Value::Function(fr) => write!(f, "<fun {:?}>", fr),
            Value::Closure(_) => write!(f, "<closure>"),
            Value::Native(_) => write!(f, "<native>")
        }
    }
}

/// Référence vers une fonction bytecode (index + arité attendue facultative).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncRef {
    /// Index dans une table de fonctions du bytecode (ex: `FuncIx` côté core).
    pub index: u32,
    /// Arité si connue (sinon `None`). Purement indicatif pour le *debug*.
    pub arity: Option<u8>,
}

impl FuncRef {
    /// Crée une nouvelle référence.
    pub const fn new(index: u32, arity: Option<u8>) -> Self { Self { index, arity } }
}

/// Upvalue capturé (simplifié : on capture la valeur par clone).
#[derive(Debug, Clone)]
pub struct Upvalue { value: Value }

/// Fermeture : fonction + upvalues.
#[derive(Debug, Clone)]
pub struct Closure {
    /// Fonction cible dans le bytecode.
    pub func: FuncRef,
    /// Valeurs capturées.
    pub upvalues: Vec<Upvalue>,
}

/// Frame d’appel.
#[derive(Debug, Clone)]
struct CallFrame {
    /// Compteur d’instructions local à ce frame (index dans `chunk.ops`).
    ip: usize,
    /// Base de pile (offset) au moment de l’appel.
    base: usize,
    /// Pour debug : fonction courante.
    func: Option<FuncRef>,
}

impl CallFrame {
    fn new(ip: usize, base: usize, func: Option<FuncRef>) -> Self { Self { ip, base, func } }
}

/// Environnement *host* pour I/O, horloge, etc.
pub trait Host: 'static {
    /// Impression utilisateur (ex: `print`).
    fn print(&mut self, s: &str);
    /// Horodatage haute résolution.
    fn now(&mut self) -> Instant { Instant::now() }
}

/// Implémentation *host* par défaut (stdout/Instant du système).
#[derive(Default)]
pub struct DefaultHost;
impl Host for DefaultHost {
    fn print(&mut self, s: &str) { println!("{s}"); }
}

/// Machine virtuelle.
pub struct Vm {
    /// Pile d’évaluation.
    stack: Vec<Value>,
    /// Pile d’appels (frames).
    frames: Vec<CallFrame>,
    /// Variables globales (nom → valeur).
    globals: HashMap<String, Value>,
    /// Limiteur d’étapes (gas) optionnel.
    gas_left: Option<u64>,
    /// Tracing des opcodes.
    trace: bool,
    /// Limites configurées.
    limits: Limits,
    /// Hôte (I/O, horloge, etc.).
    host: Box<dyn Host>,
}

#[derive(Debug, Clone, Copy)]
struct Limits {
    stack: Option<usize>,
    frames: Option<usize>,
}

impl Vm {
    /// Crée une VM avec les options fournies.
    pub fn with_options(options: VmOptions) -> Self {
        let mut vm = Self {
            stack: Vec::with_capacity(1024),
            frames: Vec::with_capacity(64),
            globals: HashMap::new(),
            gas_left: options.gas_limit,
            trace: options.trace,
            limits: Limits { stack: options.stack_limit, frames: options.call_stack_limit },
            host: Box::<DefaultHost>::default(),
        };
        if options.stdlib { vm.install_stdlib(); }
        vm
    }

    /// Crée une VM avec des options par défaut.
    pub fn new() -> Self { Self::with_options(VmOptions::default()) }

    /// Installe un hôte personnalisé.
    pub fn with_host(mut self, host: Box<dyn Host>) -> Self { self.host = host; self }

    /// Ajoute des fonctions natives de base : `print`, `clock_ms`.
    pub fn install_stdlib(&mut self) {
        self.define_native("print", |vm, args| {
            for (i, v) in args.iter().enumerate() {
                if i > 0 { vm.host.print(" "); }
                vm.host.print(&format!("{}", v));
            }
            Ok(Value::Unit)
        });
        self.define_native("clock_ms", |_vm, _| {
            let ms = Instant::now().elapsed().as_millis() as i64; // relatif au process
            Ok(Value::Int(ms))
        });
    }

    /// Déclare une globale.
    pub fn define_global(&mut self, name: impl Into<String>, val: Value) { self.globals.insert(name.into(), val); }
    /// Récupère une globale.
    pub fn get_global(&self, name: &str) -> Option<&Value> { self.globals.get(name) }
    /// Déclare une fonction native.
    pub fn define_native(&mut self, name: impl Into<String>, f: NativeFn) {
        self.define_global(name, Value::Native(f));
    }

    /// Empile une valeur (avec vérification de limite).
    fn push(&mut self, v: Value) -> VmResult<()> {
        if let Some(max) = self.limits.stack { if self.stack.len() >= max { return Err(VmError::StackOverflow); } }
        self.stack.push(v); Ok(())
    }

    /// Dépile une valeur.
    fn pop(&mut self) -> VmResult<Value> { self.stack.pop().ok_or(VmError::StackUnderflow) }

    /// Pic (lecture) en relative à la fin de pile.
    fn peek(&self, depth_from_top: usize) -> VmResult<&Value> {
        self.stack.get(self.stack.len().saturating_sub(1 + depth_from_top)).ok_or(VmError::StackUnderflow)
    }

    /// Démarre l’exécution d’un `Chunk`.
    ///
    /// Tant que vous n’avez pas fourni d’implémentation d’opcodes via [`OpAdapter`],
    /// cette fonction retournera `VmError::Unsupported` au premier opcode rencontré.
    pub fn run(&mut self, chunk: &Chunk) -> VmResult<Value> {
        self.frames.clear();
        self.frames.push(CallFrame::new(0, 0, None));
        let mut last = Value::Unit;

        loop {
            // Limitation gas
            if let Some(g) = self.gas_left.as_mut() {
                if *g == 0 { return Err(VmError::OutOfGas); }
                *g -= 1;
            }

            // Fin si plus de frames
            let frame = match self.frames.last_mut() { Some(f) => f, None => break };
            if frame.ip >= chunk.ops.len() { break; }

            let op: &Op = &chunk.ops[frame.ip];
            if self.trace { eprintln!("[ip={:04}] {:?}", frame.ip, OpDebug(op)); }
            frame.ip += 1;

            // Délègue l’exécution au *trait* OpAdapter.
            op.step(self, chunk)?;

            // NOTE: `last` peut être mis à jour par certaines opérations (ex: `Return`).
            // Ici, on n’impose rien — les handlers de vos opcodes pilotent la pile.

            // Condition de sortie facultative : si le handler a vidé toutes les frames
            if self.frames.is_empty() { break; }
        }

        // S’il reste une valeur au sommet de pile, on la renvoie.
        if let Some(v) = self.stack.last().cloned() { last = v; }
        Ok(last)
    }

    // ---- Helpers arithmétiques typés (pour vos opcodes) --------------------

    /// (Int, Int) → Int
    pub fn bin_int_int<F>(&mut self, f: F) -> VmResult<()>
    where F: FnOnce(i64, i64) -> i64 {
        let b = self.pop()?.expect_int()?;
        let a = self.pop()?.expect_int()?;
        self.push(Value::Int(f(a, b)))
    }

    /// (Float, Float) → Float
    pub fn bin_float_float<F>(&mut self, f: F) -> VmResult<()>
    where F: FnOnce(f64, f64) -> f64 {
        let b = self.pop()?.expect_float()?;
        let a = self.pop()?.expect_float()?;
        self.push(Value::Float(f(a, b)))
    }

    /// Appelle une fonction *native* déjà au sommet de pile, en lui passant `argc` args.
    pub fn call_native_on_stack(&mut self, argc: usize) -> VmResult<()> {
        let func = self.peek(argc)?.clone();
        let args_start = self.stack.len() - argc;
        let args = &self.stack[args_start..];
        match func {
            Value::Native(f) => {
                let ret = f(self, args)?;
                for _ in 0..=argc { self.stack.pop(); } // enlève fn + args
                self.push(ret)
            }
            other => Err(VmError::TypeError(format!("appel d’un non-fonction: {other:?}")))
        }
    }

    /// Empile un nouvel appel (bytecode) — crée un frame d’appel.
    pub fn push_call(&mut self, target_ip: usize, func: Option<FuncRef>) -> VmResult<()> {
        if let Some(max) = self.limits.frames { if self.frames.len() >= max { return Err(VmError::CallStackOverflow); } }
        let base = self.stack.len();
        self.frames.push(CallFrame::new(target_ip, base, func));
        Ok(())
    }

    /// Retourne du frame courant avec `retc` valeurs retournées (par défaut 1).
    pub fn return_from_call(&mut self, retc: usize) -> VmResult<()> {
        let frame = self.frames.pop().ok_or(VmError::CallStackUnderflow)?;
        // On garde les `retc` dernières valeurs et on nettoie la pile jusqu’à `frame.base`.
        let retvals: Vec<Value> = (0..retc).map(|_| self.pop()).collect::<VmResult<Vec<_>>>()?;
        self.stack.truncate(frame.base);
        for v in retvals.into_iter().rev() { self.push(v)?; }
        Ok(())
    }
}

/// Aides de *pattern matching* et conversions typées.
impl Value {
    /// Attend un entier, sinon `TypeError`.
    pub fn expect_int(self) -> VmResult<i64> { match self { Value::Int(i) => Ok(i), x => Err(VmError::TypeError(format!("attendu Int, eu {x:?}"))) } }
    /// Attend un float, sinon `TypeError`.
    pub fn expect_float(self) -> VmResult<f64> { match self { Value::Float(x) => Ok(x), x => Err(VmError::TypeError(format!("attendu Float, eu {x:?}"))) } }
    /// Attend une chaîne, sinon `TypeError`.
    pub fn expect_str(self) -> VmResult<Gc<String>> { match self { Value::Str(s) => Ok(s), x => Err(VmError::TypeError(format!("attendu Str, eu {x:?}"))) } }
}

/// Erreurs de la VM.
#[derive(Debug)]
pub enum VmError {
    /// Empilement trop profond.
    StackOverflow,
    /// Dépiler alors que la pile est vide.
    StackUnderflow,
    /// Trop de frames d’appel.
    CallStackOverflow,
    /// Retour sans frame.
    CallStackUnderflow,
    /// Type inattendu.
    TypeError(String),
    /// Opcode non supporté par l’adaptateur.
    Unsupported(String),
    /// Exécution trop longue (gas épuisé).
    OutOfGas,
    /// Autre erreur utilisateur.
    Other(String),
}

impl Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmError::StackOverflow => write!(f, "stack overflow"),
            VmError::StackUnderflow => write!(f, "stack underflow"),
            VmError::CallStackOverflow => write!(f, "call stack overflow"),
            VmError::CallStackUnderflow => write!(f, "call stack underflow"),
            VmError::TypeError(s) => write!(f, "type error: {s}"),
            VmError::Unsupported(op) => write!(f, "unsupported opcode: {op}"),
            VmError::OutOfGas => write!(f, "out of gas"),
            VmError::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for VmError {}

/// Affichage *Debug* sans exiger `Op: Debug` directement dans les bounds publics.
struct OpDebug<'a, T>(&'a T);
impl<'a, T: Debug> Debug for OpDebug<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { Debug::fmt(self.0, f) }
}

// =====================================================================================
//  Adapteur d’opcodes (trait à implémenter pour votre type `Op`)
// =====================================================================================

/// Trait qui permet à la VM d’exécuter votre type d’opcodes.
///
/// C’est volontairement minimal : **un seul point d’entrée** `step` qui reçoit
/// la VM et le `Chunk` courant. Vous êtes libre de manipuler la pile via les
/// méthodes de [`Vm`].
pub trait OpAdapter {
    /// Exécute une étape (un opcode). Renvoie `Ok(())` si tout va bien.
    fn step(&self, vm: &mut Vm, chunk: &Chunk) -> VmResult<()>;
}

/// Implémentation par défaut : **non supporté**.
impl OpAdapter for Op {
    fn step(&self, _vm: &mut Vm, _chunk: &Chunk) -> VmResult<()> {
        Err(VmError::Unsupported(format!("{self:?}")))
    }
}

// =====================================================================================
//  Utilitaires & mini-stdlib
// =====================================================================================

/// Construit une `Value::Str` à partir d’un `String`.
pub fn vstr<S: Into<String>>(s: S) -> Value { Value::Str(Rc::new(RefCell::new(s.into()))) }

/// Construit une `Value::Array` vide.
pub fn varray() -> Value { Value::Array(Rc::new(RefCell::new(Vec::new()))) }

/// Construit une `Value::Map` vide.
pub fn vmap() -> Value { Value::Map(Rc::new(RefCell::new(HashMap::new()))) }

// =====================================================================================
//  Tests basiques (n’utilisent pas d’opcodes spécifiques)
// =====================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_push_pop() {
        let mut vm = Vm::new();
        vm.push(Value::Int(1)).unwrap();
        vm.push(Value::Int(2)).unwrap();
        assert_eq!(vm.pop().unwrap().expect_int().unwrap(), 2);
        assert_eq!(vm.pop().unwrap().expect_int().unwrap(), 1);
    }

    #[test]
    fn native_print_exists_when_stdlib_enabled() {
        let mut vm = Vm::with_options(VmOptions::default().with_stdlib(true));
        assert!(matches!(vm.get_global("print"), Some(Value::Native(_))));
    }

    // Teste que run() s’arrête proprement quand chunk.ops est vide.
    #[test]
    fn run_empty_chunk_ok() {
        let mut vm = Vm::new();
        let chunk = Chunk { ops: Vec::<Op>::new() };
        let out = vm.run(&chunk).unwrap();
        match out { Value::Unit => {}, _ => panic!("attendu Unit") }
    }
}
