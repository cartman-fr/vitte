//! vitte-vm/util.rs
//!
//! Outils transverses pour vitte-vm : IO little-endian, CRC32, hachages,
//! varints, alignement, dumps hex, etc. Pensé pour être no_std (+alloc) friendly.

#![forbid(unsafe_code)]

#[cfg(all(not(feature = "std"), not(test)))]
extern crate core as std_core;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{string::String, vec::Vec, borrow::ToOwned};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use core::{fmt, mem};

// ==============================
// Alignement & bornes
// ==============================

/// Aligne `x` vers le haut au multiple `to` (puissance de deux recommandée).
#[inline]
pub const fn align_up(x: usize, to: usize) -> usize {
    if to == 0 { x } else { (x + (to - 1)) / to * to }
}

/// Renvoie une erreur si `val > max` (utile contre fichiers malicieux).
#[inline]
pub fn guard_u32(val: u32, max: u32, label: &'static str) -> Result<(), GuardError> {
    if val > max { Err(GuardError { label, val, max }) } else { Ok(()) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardError {
    pub label: &'static str,
    pub val: u32,
    pub max: u32,
}
impl fmt::Display for GuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={} > max={}", self.label, self.val, self.max)
    }
}

// ==============================
// Little Endian: lecture / écriture
// ==============================

/// Curseur de lecture Little-Endian sur slice (sans alloc).
pub struct CursorLE<'a> {
    buf: &'a [u8],
    off: usize,
}
impl<'a> CursorLE<'a> {
    pub fn new(buf: &'a [u8]) -> Self { Self { buf, off: 0 } }
    #[inline] pub fn remaining(&self) -> usize { self.buf.len().saturating_sub(self.off) }
    #[inline] pub fn pos(&self) -> usize { self.off }
    #[inline] pub fn is_eof(&self) -> bool { self.off >= self.buf.len() }

    #[inline]
    pub fn read_exact(&mut self, n: usize) -> Result<&'a [u8], IoSliceError> {
        if self.off + n > self.buf.len() {
            return Err(IoSliceError::Eof { want: n, have: self.buf.len().saturating_sub(self.off) });
        }
        let s = &self.buf[self.off..self.off + n];
        self.off += n;
        Ok(s)
    }

    #[inline] pub fn read_u8(&mut self)  -> Result<u8,  IoSliceError> { Ok(self.read_exact(1)?[0]) }
    #[inline] pub fn read_u16(&mut self) -> Result<u16, IoSliceError> { Ok(u16::from_le_bytes(self.read_exact(2)?.try_into().unwrap())) }
    #[inline] pub fn read_u32(&mut self) -> Result<u32, IoSliceError> { Ok(u32::from_le_bytes(self.read_exact(4)?.try_into().unwrap())) }
    #[inline] pub fn read_u64(&mut self) -> Result<u64, IoSliceError> { Ok(u64::from_le_bytes(self.read_exact(8)?.try_into().unwrap())) }
    #[inline] pub fn read_i64(&mut self) -> Result<i64, IoSliceError> { Ok(i64::from_le_bytes(self.read_exact(8)?.try_into().unwrap())) }

    /// Lit un nom (len u16 + bytes UTF-8).
    pub fn read_name(&mut self, max_len: usize) -> Result<String, NameError> {
        let len = self.read_u16()? as usize;
        if len > max_len { return Err(NameError::TooLong { len, max: max_len }); }
        let s = self.read_exact(len).map_err(|e| NameError::Io(e))?;
        match core::str::from_utf8(s) {
            Ok(t) => Ok(t.to_owned()),
            Err(_) => Err(NameError::Utf8),
        }
    }

    /// Récupère le reste des octets (avance à la fin).
    pub fn take_rest(&mut self) -> &'a [u8] {
        let s = &self.buf[self.off..];
        self.off = self.buf.len();
        s
    }
}

/// Buffer d’écriture Little-Endian (simple wrapper sur Vec<u8>).
pub struct BufLE {
    pub buf: Vec<u8>,
}
impl BufLE {
    pub fn with_capacity(n: usize) -> Self { Self { buf: Vec::with_capacity(n) } }
    pub fn into_inner(self) -> Vec<u8> { self.buf }
    #[inline] pub fn write_u8(&mut self, v: u8)  { self.buf.push(v) }
    #[inline] pub fn write_u16(&mut self, v: u16) { self.buf.extend_from_slice(&v.to_le_bytes()) }
    #[inline] pub fn write_u32(&mut self, v: u32) { self.buf.extend_from_slice(&v.to_le_bytes()) }
    #[inline] pub fn write_u64(&mut self, v: u64) { self.buf.extend_from_slice(&v.to_le_bytes()) }
    #[inline] pub fn write_i64(&mut self, v: i64) { self.buf.extend_from_slice(&v.to_le_bytes()) }

    /// Écrit un nom (len u16 + bytes).
    pub fn write_name(&mut self, s: &str, max_len: usize) -> Result<(), NameError> {
        let bytes = s.as_bytes();
        if bytes.len() > max_len { return Err(NameError::TooLong { len: bytes.len(), max: max_len }); }
        self.write_u16(bytes.len() as u16);
        self.buf.extend_from_slice(bytes);
        Ok(())
    }
}

#[derive(Debug)]
pub enum IoSliceError {
    Eof { want: usize, have: usize },
}
impl fmt::Display for IoSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self { IoSliceError::Eof { want, have } => write!(f, "EOF: want={}, have={}", want, have) }
    }
}

#[derive(Debug)]
pub enum NameError {
    Io(IoSliceError),
    Utf8,
    TooLong { len: usize, max: usize },
}
impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NameError::Io(e) => write!(f, "io: {e}"),
            NameError::Utf8 => write!(f, "nom non UTF-8"),
            NameError::TooLong { len, max } => write!(f, "nom trop long: {} > {}", len, max),
        }
    }
}

// ==============================
// CRC32 (IEEE) & FNV-1a 64
// ==============================

/// CRC32 (IEEE 802.3, polynôme 0xEDB88320).
pub fn crc32_ieee(data: &[u8]) -> u32 {
    const POLY: u32 = 0xEDB88320;
    static INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
    static mut TABLE: [u32; 256] = [0; 256];

    unsafe {
        if !INIT.load(core::sync::atomic::Ordering::Relaxed) {
            // build table une fois (race bénigne si doublon identique)
            for i in 0..256u32 {
                let mut c = i;
                for _ in 0..8 { c = if (c & 1) != 0 { POLY ^ (c >> 1) } else { c >> 1 }; }
                TABLE[i as usize] = c;
            }
            INIT.store(true, core::sync::atomic::Ordering::Relaxed);
        }
        let mut c: u32 = 0xFFFF_FFFF;
        for &b in data {
            c = TABLE[((c ^ (b as u32)) & 0xFF) as usize] ^ (c >> 8);
        }
        !c
    }
}

/// FNV-1a 64 déterministe (utile pour “index” de constantes symboliques).
pub fn fnv1a64(s: &str) -> u64 {
    const OFF: u64 = 0xcbf29ce484222325;
    const PRM: u64 = 0x100000001b3;
    let mut h = OFF;
    for &b in s.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(PRM);
    }
    h
}

// ==============================
// Varints (ULEB128 / SLEB128)
// ==============================

/// Encode un `u64` en ULEB128.
pub fn uleb128_encode(mut v: u64, out: &mut Vec<u8>) {
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 { byte |= 0x80; }
        out.push(byte);
        if v == 0 { break; }
    }
}

/// Décode un `u64` ULEB128, renvoie (valeur, octets_lus).
pub fn uleb128_decode(buf: &[u8]) -> Result<(u64, usize), VarintError> {
    let mut res: u64 = 0;
    let mut shift = 0;
    for (i, &b) in buf.iter().enumerate() {
        let part = (b & 0x7F) as u64;
        res |= part << shift;
        if (b & 0x80) == 0 { return Ok((res, i + 1)); }
        shift += 7;
        if shift >= 64 { return Err(VarintError::Overflow); }
    }
    Err(VarintError::Eof)
}

/// Encode un `i64` en SLEB128.
pub fn sleb128_encode(mut v: i64, out: &mut Vec<u8>) {
    loop {
        let byte = (v as u8) & 0x7F;
        let sign = if (v & !0x7F) == 0 || (v & !0x7F) == !0 { ((v >> 6) & 1) as u8 } else { 0 };
        let done = ((v == 0) && (sign == 0)) || ((v == -1) && (sign == 1));
        let mut b = byte;
        if !done { b |= 0x80; }
        out.push(b);
        if done { break; }
        v >>= 7;
    }
}

/// Décode un `i64` SLEB128, renvoie (valeur, octets_lus).
pub fn sleb128_decode(buf: &[u8]) -> Result<(i64, usize), VarintError> {
    let mut res: i64 = 0;
    let mut shift = 0;
    let mut byte;
    let mut i = 0;
    loop {
        if i >= buf.len() { return Err(VarintError::Eof); }
        byte = buf[i];
        res |= (((byte & 0x7F) as i64) << shift);
        shift += 7;
        i += 1;
        if (byte & 0x80) == 0 { break; }
        if shift >= 64 { return Err(VarintError::Overflow); }
    }
    // signe
    if shift < 64 && (byte & 0x40) != 0 {
        res |= (!0i64) << shift;
    }
    Ok((res, i))
}

#[derive(Debug)]
pub enum VarintError { Eof, Overflow }
impl fmt::Display for VarintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self { VarintError::Eof => write!(f, "eof"), VarintError::Overflow => write!(f, "overflow") }
    }
}

// ==============================
// Formatage & dumps
// ==============================

/// Échappe une chaîne UTF-8 pour affichage ASM/JSON simple.
pub fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"'  => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let u = c as u32;
                let _ = core::fmt::Write::write_fmt(&mut out, format_args!("\\u{:04X}", u));
            }
            c => out.push(c),
        }
    }
    out
}

/// Dump hex multi-ligne lisible (adresse + 16 octets + ASCII).
pub fn hex_dump(bytes: &[u8], start_addr: usize) -> String {
    const W: usize = 16;
    let mut s = String::new();
    for (i, chunk) in bytes.chunks(W).enumerate() {
        let addr = start_addr + i * W;
        let _ = fmt::Write::write_fmt(&mut s, format_args!("{:08X}  ", addr));
        for j in 0..W {
            if j < chunk.len() {
                let _ = fmt::Write::write_fmt(&mut s, format_args!("{:02X} ", chunk[j]));
            } else {
                s.push_str("   ");
            }
            if j == 7 { s.push(' '); }
        }
        s.push(' ');
        for &b in chunk {
            let c = if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' };
            s.push(c);
        }
        s.push('\n');
    }
    s
}

// ==============================
// Chrono simple (std only)
// ==============================

#[cfg(feature = "std")]
pub struct Timer(std::time::Instant);
#[cfg(feature = "std")]
impl Timer {
    pub fn start() -> Self { Self(std::time::Instant::now()) }
    pub fn elapsed_ms(&self) -> u128 { self.0.elapsed().as_millis() }
    pub fn elapsed_us(&self) -> u128 { self.0.elapsed().as_micros() }
}

// ==============================
// Tests
// ==============================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_works() {
        assert_eq!(align_up(0, 4), 0);
        assert_eq!(align_up(1, 4), 4);
        assert_eq!(align_up(16, 8), 16);
    }

    #[test]
    fn fnv_is_deterministic() {
        assert_eq!(fnv1a64("abc"), fnv1a64("abc"));
        assert_ne!(fnv1a64("abc"), fnv1a64("abd"));
    }

    #[test]
    fn crc_nonzero() {
        let c = crc32_ieee(b"hello");
        assert_ne!(c, 0);
    }

    #[test]
    fn varint_roundtrip() {
        let mut v = Vec::new();
        uleb128_encode(624485, &mut v); // exemple canonique DWARF
        let (x, n) = uleb128_decode(&v).unwrap();
        assert_eq!(x, 624485);
        assert_eq!(n, v.len());

        let mut v2 = Vec::new();
        sleb128_encode(-123456, &mut v2);
        let (y, m) = sleb128_decode(&v2).unwrap();
        assert_eq!(y, -123456);
        assert_eq!(m, v2.len());
    }

    #[test]
    fn cursor_and_buf() {
        let mut b = BufLE::with_capacity(64);
        b.write_u8(7);
        b.write_u16(0xBEEF);
        b.write_u32(0xDEAD_BEEF);
        b.write_i64(-42);

        let mut cur = CursorLE::new(&b.buf);
        assert_eq!(cur.read_u8().unwrap(), 7);
        assert_eq!(cur.read_u16().unwrap(), 0xBEEF);
        assert_eq!(cur.read_u32().unwrap(), 0xDEAD_BEEF);
        assert_eq!(cur.read_i64().unwrap(), -42);
        assert!(cur.is_eof());
    }

    #[test]
    fn dump_and_escape() {
        let _ = hex_dump(b"Hello,\nVitte!", 0);
        let esc = escape_str("a\"b\\c\nd");
        assert!(esc.contains("\\\""));
        assert!(esc.contains("\\\\"));
        assert!(esc.contains("\\n"));
    }
}
