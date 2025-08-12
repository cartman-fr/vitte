use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn ensure_parent(p: &Path) -> io::Result<()> {
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

pub fn read_to_bytes(p: &Path) -> io::Result<Vec<u8>> {
    fs::read(p)
}

pub fn write_all(p: &Path, bytes: &[u8]) -> io::Result<()> {
    ensure_parent(p)?;
    fs::write(p, bytes)
}

pub fn tmp_file(prefix: &str, ext: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    p.push(format!("{}-{}.{}", prefix, ts, ext.trim_start_matches('.')));
    p
}