//! Bytecode core for Vitte: opcodes, chunk format, helpers.
//! Re-export pour usage simple ailleurs.

pub mod op;
pub mod chunk;

pub use op::Op;
pub use chunk::{Chunk, ChunkFlags, ConstPool, ConstValue, LineTable};
