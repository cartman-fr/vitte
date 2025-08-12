#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Int, Float, Str, Bool,
    List(Box<Ty>),
    Fn(Vec<Ty>, Box<Ty>),
    Var(u32),
    Unknown,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Scheme { pub vars: Vec<u32>, pub ty: Ty }
