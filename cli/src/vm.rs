
use color_eyre::eyre::{Result, eyre};
use std::path::Path;
use crate::vm;
use crate::bytecode::Op;

pub fn run(file: &Path) -> Result<()> {
    let bin = std::fs::read(file)?;
    let ops: Vec<Op> = bincode::deserialize(&bin).map_err(|e| eyre!("bytecode invalide: {}", e))?;
    let chunk = crate::bytecode::Chunk{ ops };
    vm::run(&chunk)
}
