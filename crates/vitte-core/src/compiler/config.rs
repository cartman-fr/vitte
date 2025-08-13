//! config.rs — Configuration “core compiler/codegen” pour Vitte.
//!
//! Objectif : fournir un **noyau de configuration** minimal (sans dépendances externes)
//! partagé par les composants proches du bytecode (codegen, assembleur, outils).
//!
//! - Defaults sûrs (`Config::default()`)
//! - Lecture **ENV** (préfixe `VITTE_CORE_...`) via `Config::from_env()`
//! - **Overrides CLI** via `CliOverrides` (appliqués avec `apply_cli_overrides`)
//! - Limites de sécurité (nb d’opcodes max, pool de constantes, arité, etc.)
//! - Helpers : `flags()` → `ChunkFlags`, `validate()`…
//!
//! ENV supportés (tous facultatifs) :
//!   VITTE_CORE_OPT=O0|O1|O2|O3|Os|Oz
//!   VITTE_CORE_DEBUG=none|line|full
//!   VITTE_CORE_COLOR=auto|always|never
//!   VITTE_CORE_WARN=allow|warn|deny
//!   VITTE_CORE_STRIP=0|1
//!   VITTE_CORE_DEDUP=0|1
//!   VITTE_CORE_ENDIAN=native|little|big
//!   VITTE_CORE_MAX_OPS=<usize>
//!   VITTE_CORE_MAX_CONSTS=<usize>
//!   VITTE_CORE_MAX_ARITY=<u8>
//!   VITTE_CORE_MAX_STRLEN=<usize>
//!   VITTE_CORE_ALIGN_CONST=0|1
//!   VITTE_CORE_VERIFY_RT=0|1
//!
//! NB: pas de parsing TOML ici (garde `vitte-core` léger). Les crates outils
//!     peuvent sérialiser/désérialiser via la feature `serde`.

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, unused_must_use)]

use crate::bytecode::chunk::ChunkFlags;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/* ─────────────────────────── Types publics ─────────────────────────── */

/// Niveau d’optimisation "core" (influence passes locales / const-folding, etc.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OptLevel { O0, O1, O2, O3, Os, Oz }

/// Granularité des infos de debug dans le bytecode généré.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DebugInfo { None, Line, Full }

/// Politique d’avertissements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WarningsAs { Allow, Warn, Deny }

/// Mode couleur (pour diagnostics / désasm *dans les outils*).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColorMode { Auto, Always, Never }

/// Endianness ciblée à l’émission (utile si tu stabilises un format multi-arch).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Endianness { Native, Little, Big }

/// Limites de sûreté pour la génération/validation de chunks.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Limits {
    /// Nombre maximal d’opcodes autorisé dans un chunk.
    pub max_ops: usize,
    /// Taille maximale du pool de constantes.
    pub max_consts: usize,
    /// Arité maximale d’un appel (protection).
    pub max_func_arity: u8,
    /// Longueur maximale d’une constante string.
    pub max_string_len: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_ops: 1_000_000,
            max_consts: 65_536,
            max_func_arity: u8::MAX,
            max_string_len: 1_000_000,
        }
    }
}

/// Paramétrage codegen/émission de bytecode.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Codegen {
    /// Dé-duplication du pool de constantes.
    pub dedup_consts: bool,
    /// Strip des infos de debug lors de l’émission.
    pub strip_debug: bool,
    /// Alignement/rangement du pool (cosmétique/IO-friendly).
    pub align_constants: bool,
    /// Endianness visée (si applicable).
    pub target_endianness: Endianness,
    /// Vérifier le round-trip (to_bytes → from_bytes) en mode debug/test.
    pub verify_roundtrip: bool,
}
impl Default for Codegen {
    fn default() -> Self {
        Self {
            dedup_consts: true,
            strip_debug: false,
            align_constants: true,
            target_endianness: Endianness::Native,
            verify_roundtrip: false,
        }
    }
}

/// Configuration “core compiler/codegen” complète.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    /* Général */
    pub opt_level: OptLevel,
    pub debug_info: DebugInfo,
    pub warnings: WarningsAs,
    pub color: ColorMode,

    /* Limites & Codegen */
    pub limits: Limits,
    pub codegen: Codegen,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            opt_level: OptLevel::O1,
            debug_info: DebugInfo::Line,
            warnings: WarningsAs::Warn,
            color: ColorMode::Auto,
            limits: Limits::default(),
            codegen: Codegen::default(),
        }
    }
}

/* ─────────────────────── Overrides (CLI / couches) ─────────────────────── */

/// Overrides typiques fournis par une CLI en amont.
/// Toutes les valeurs sont optionnelles — applique-les avec `apply_cli_overrides`.
#[derive(Default, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CliOverrides {
    pub opt_level: Option<OptLevel>,
    pub debug_info: Option<DebugInfo>,
    pub warnings: Option<WarningsAs>,
    pub color: Option<ColorMode>,

    pub dedup_consts: Option<bool>,
    pub strip_debug: Option<bool>,
    pub align_constants: Option<bool>,
    pub target_endianness: Option<Endianness>,
    pub verify_roundtrip: Option<bool>,

    pub max_ops: Option<usize>,
    pub max_consts: Option<usize>,
    pub max_func_arity: Option<u8>,
    pub max_string_len: Option<usize>,
}

impl Config {
    /// Construit depuis les valeurs par défaut + ENV.
    pub fn from_env() -> Self {
        let mut c = Self::default();
        c.apply_env();
        c
    }

    /// Applique les variables d’environnement `VITTE_CORE_*`.
    pub fn apply_env(&mut self) {
        // enums
        if let Some(v) = read_env("VITTE_CORE_OPT")       { if let Some(e) = parse_opt(&v)        { self.opt_level = e; } }
        if let Some(v) = read_env("VITTE_CORE_DEBUG")     { if let Some(e) = parse_debug(&v)      { self.debug_info = e; } }
        if let Some(v) = read_env("VITTE_CORE_COLOR")     { if let Some(e) = parse_color(&v)      { self.color = e; } }
        if let Some(v) = read_env("VITTE_CORE_WARN")      { if let Some(e) = parse_warn(&v)       { self.warnings = e; } }
        if let Some(v) = read_env("VITTE_CORE_ENDIAN")    { if let Some(e) = parse_endian(&v)     { self.codegen.target_endianness = e; } }

        // bools
        if let Some(v) = read_env("VITTE_CORE_DEDUP")     { if let Some(b) = parse_bool(&v)       { self.codegen.dedup_consts = b; } }
        if let Some(v) = read_env("VITTE_CORE_STRIP")     { if let Some(b) = parse_bool(&v)       { self.codegen.strip_debug = b; } }
        if let Some(v) = read_env("VITTE_CORE_ALIGN_CONST"){ if let Some(b) = parse_bool(&v)      { self.codegen.align_constants = b; } }
        if let Some(v) = read_env("VITTE_CORE_VERIFY_RT") { if let Some(b) = parse_bool(&v)       { self.codegen.verify_roundtrip = b; } }

        // num
        if let Some(v) = read_env("VITTE_CORE_MAX_OPS")   { if let Some(n) = parse_usize(&v)      { self.limits.max_ops = n; } }
        if let Some(v) = read_env("VITTE_CORE_MAX_CONSTS"){ if let Some(n) = parse_usize(&v)      { self.limits.max_consts = n; } }
        if let Some(v) = read_env("VITTE_CORE_MAX_ARITY") { if let Ok(n) = v.parse::<u16>()        { self.limits.max_func_arity = n.min(u8::MAX as u16) as u8; } }
        if let Some(v) = read_env("VITTE_CORE_MAX_STRLEN"){ if let Some(n) = parse_usize(&v)      { self.limits.max_string_len = n; } }
    }

    /// Applique des overrides “dernier mot” typiquement issus d’une CLI.
    pub fn apply_cli_overrides(&mut self, o: &CliOverrides) {
        if let Some(x) = o.opt_level          { self.opt_level = x; }
        if let Some(x) = o.debug_info         { self.debug_info = x; }
        if let Some(x) = o.warnings           { self.warnings = x; }
        if let Some(x) = o.color              { self.color = x; }

        if let Some(x) = o.dedup_consts       { self.codegen.dedup_consts = x; }
        if let Some(x) = o.strip_debug        { self.codegen.strip_debug = x; }
        if let Some(x) = o.align_constants    { self.codegen.align_constants = x; }
        if let Some(x) = o.target_endianness  { self.codegen.target_endianness = x; }
        if let Some(x) = o.verify_roundtrip   { self.codegen.verify_roundtrip = x; }

        if let Some(x) = o.max_ops            { self.limits.max_ops = x; }
        if let Some(x) = o.max_consts         { self.limits.max_consts = x; }
        if let Some(x) = o.max_func_arity     { self.limits.max_func_arity = x; }
        if let Some(x) = o.max_string_len     { self.limits.max_string_len = x; }
    }

    /// Retourne les flags à écrire dans l’en-tête de chunk.
    pub fn flags(&self) -> ChunkFlags {
        ChunkFlags { stripped: self.codegen.strip_debug }
    }

    /// Validation de base (retourne `Err(&'static str)` si incohérence).
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.limits.max_func_arity == 0 { return Err("max_func_arity doit être > 0"); }
        if self.limits.max_ops == 0        { return Err("max_ops doit être > 0"); }
        if self.limits.max_consts == 0     { return Err("max_consts doit être > 0"); }
        Ok(())
    }
}

/* ────────────────────────── Parsing d’ENV ────────────────────────── */

fn read_env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no"  | "n" | "off"=> Some(false),
        _ => None,
    }
}
fn parse_usize(s: &str) -> Option<usize> {
    s.trim().parse::<usize>().ok()
}
fn parse_opt(s: &str) -> Option<OptLevel> {
    match s.trim().to_ascii_uppercase().as_str() {
        "O0" => Some(OptLevel::O0),
        "O1" => Some(OptLevel::O1),
        "O2" => Some(OptLevel::O2),
        "O3" => Some(OptLevel::O3),
        "OS" => Some(OptLevel::Os),
        "OZ" => Some(OptLevel::Oz),
        _ => None,
    }
}
fn parse_debug(s: &str) -> Option<DebugInfo> {
    match s.trim().to_ascii_lowercase().as_str() {
        "none" => Some(DebugInfo::None),
        "line" => Some(DebugInfo::Line),
        "full" => Some(DebugInfo::Full),
        _ => None,
    }
}
fn parse_color(s: &str) -> Option<ColorMode> {
    match s.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(ColorMode::Auto),
        "always" => Some(ColorMode::Always),
        "never" => Some(ColorMode::Never),
        _ => None,
    }
}
fn parse_warn(s: &str) -> Option<WarningsAs> {
    match s.trim().to_ascii_lowercase().as_str() {
        "allow" => Some(WarningsAs::Allow),
        "warn"  => Some(WarningsAs::Warn),
        "deny"  => Some(WarningsAs::Deny),
        _ => None,
    }
}
fn parse_endian(s: &str) -> Option<Endianness> {
    match s.trim().to_ascii_lowercase().as_str() {
        "native" => Some(Endianness::Native),
        "little" => Some(Endianness::Little),
        "big"    => Some(Endianness::Big),
        _ => None,
    }
}

/* ───────────────────────────── Tests ───────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let c = Config::default();
        assert!(matches!(c.opt_level, OptLevel::O1));
        assert!(matches!(c.debug_info, DebugInfo::Line));
        assert!(c.limits.max_ops > 10_000);
        assert!(c.codegen.dedup_consts);
        assert!(!c.codegen.strip_debug);
    }

    #[test]
    fn flags_reflect_strip() {
        let mut c = Config::default();
        assert_eq!(c.flags().stripped, false);
        c.codegen.strip_debug = true;
        assert_eq!(c.flags().stripped, true);
    }

    #[test]
    fn cli_overrides_last_word() {
        let mut c = Config::default();
        let mut o = CliOverrides::default();
        o.strip_debug = Some(true);
        o.max_ops = Some(42);
        o.opt_level = Some(OptLevel::O3);
        c.apply_cli_overrides(&o);
        assert!(c.codegen.strip_debug);
        assert_eq!(c.limits.max_ops, 42);
        assert!(matches!(c.opt_level, OptLevel::O3));
    }

    #[test]
    fn endian_parse() {
        assert!(matches!(parse_endian("little"), Some(Endianness::Little)));
        assert!(parse_endian("weird").is_none());
    }

    #[test]
    fn validate_limits() {
        let mut c = Config::default();
        c.limits.max_ops = 0;
        assert!(c.validate().is_err());
    }
}
