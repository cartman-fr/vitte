pub mod bytecode_cli;

#[derive(Debug, Clone, Copy)]
pub enum BackendKind {
    /// Délègue au binaire `vitte` local pour produire du bytecode .vbc
    BytecodeCli,
}