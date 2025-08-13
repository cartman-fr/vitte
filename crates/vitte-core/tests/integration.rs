//! tests/integration.rs — batteries d’intégration pour vitte-vm
//!
//! Hypothèses :
//! - Le crate s’appelle `vitte-vm` côté Cargo, donc import via `vitte_vm::...`.
//! - Les modules exposés : `asm`, `loader`, (et optionnellement `util` si exporté).
//!
//! Astuce : lance en local avec :
//!   cargo test -p vitte-vm
//!   cargo test -p vitte-vm --features zstd

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use vitte_vm::{
    asm::{self, OpcodeTable, OpArgKind, OpSig, RawOp, RawProgram},
    loader,
};

// -----------------------------------------------------------------------------
// Helpers de test
// -----------------------------------------------------------------------------

fn temp_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!("vittevm_test_{pid}_{nanos}_{name}"));
    p
}

fn sample_asm() -> &'static str {
    r#"
        ; Démo d'intégration complète
        .entry main
        .const PI = 3.14159
        .string msg = "Hello, Vitte!"
        .data db = 1, 2, 3, 4
        .org 4096
        .data 255, 0, 255

    main:
        NOP
        LOADI r0, 42
        LOADK r1, const:PI
        CALL  r2, r1, 0
        JZ    r2, @end
        ADD   r3, r0, r1
    end:
        RET
    "#
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[test]
fn assemble_then_save_and_reload_uncompressed() {
    // 1) Assemble
    let assembled = asm::assemble(sample_asm()).expect("assemble ok");
    let prog = assembled.program;

    // 2) Sauvegarde en binaire non compressé
    let path = temp_path("uncompressed.vitbc");
    loader::save_raw_program_to_path(&path, &prog, false).expect("save ok");

    // 3) Rechargement
    let back = loader::load_raw_program_from_path(&path).expect("load ok");

    // 4) Vérifs de surface
    assert_eq!(back.entry_pc, prog.entry_pc);
    assert_eq!(back.code.len(), prog.code.len());
    assert!(!back.const_pool.ints.is_empty());
    assert!(back.const_pool.strings.get("msg").is_some());

    // 5) Désassemblage lisible
    let text = asm::disassemble(&back, &OpcodeTable::new_default());
    assert!(text.contains(".string msg"));
    assert!(text.contains("L0:"));
    assert!(text.contains("RET"));

    // Nettoyage
    let _ = fs::remove_file(&path);
}

#[cfg(feature = "zstd")]
#[test]
fn assemble_then_save_and_reload_compressed() {
    let assembled = asm::assemble(sample_asm()).expect("assemble ok");
    let prog = assembled.program;

    let path = temp_path("compressed.vitbc");
    loader::save_raw_program_to_path(&path, &prog, true).expect("save zstd ok");

    let back = loader::load_raw_program_from_path(&path).expect("load ok");
    assert_eq!(back.code.len(), prog.code.len());
    assert_eq!(back.entry_pc, prog.entry_pc);

    let _ = fs::remove_file(&path);
}

#[test]
fn loader_detects_crc_corruption() {
    // Sauvegarde en mémoire
    let assembled = asm::assemble(sample_asm()).expect("assemble ok");
    let prog = assembled.program;

    let mut buf = Vec::new();
    loader::save_raw_program(&mut buf, &prog, false).expect("save -> vec ok");

    // Corrompre un octet dans le body (après MAGIC)
    if buf.len() > loader::MAGIC.len() + 16 {
        let i = loader::MAGIC.len() + 16;
        buf[i] ^= 0xFF;
    }

    // Doit échouer sur le CRC
    let err = loader::load_raw_program(&buf[..]).unwrap_err();
    match err {
        loader::LoaderError::ChecksumMismatch { .. } => {} // OK
        other => panic!("attendu ChecksumMismatch, got {other:?}"),
    }
}

#[test]
fn opcode_table_extension_and_encode_path() {
    // Table custom : on ajoute une pseudo-instruction "INC rX"
    let mut table = OpcodeTable::new_default();
    table.insert(OpSig {
        code: 0x40,
        name: "INC",
        operands: &[OpArgKind::Reg],
    });

    // Programme ASM qui l'utilise
    let src = r#"
        entry:
            LOADI r0, 5
            INC   r0
            RET
    "#;

    // Assemble avec table custom
    let toks = asm::assemble_with_table(src, &table).expect("assemble with table ok");
    let prog = toks.program;

    // On vérifie qu'on a bien nos 3 ops
    assert_eq!(prog.code.len(), 3);

    // Round-trip binaire
    let mut buf = Vec::new();
    loader::save_raw_program(&mut buf, &prog, false).expect("save ok");
    let back = loader::load_raw_program(&buf[..]).expect("load ok");

    // Désassemble avec la même table : on doit retrouver "INC"
    let dis = asm::disassemble(&back, &table);
    assert!(dis.contains("INC r0"));
}

#[test]
fn disassemble_contains_labels_consts_and_data() {
    let out = asm::assemble(sample_asm()).expect("assemble ok");
    let dis = asm::disassemble(&out.program, &OpcodeTable::new_default());

    // Labels auto L* + .entry + nos directives
    assert!(dis.contains(".entry"));
    assert!(dis.contains(".const PI"));
    assert!(dis.contains(".string msg"));
    assert!(dis.contains(".data"));
    assert!(dis.contains("L0:"));
}

#[test]
fn full_file_roundtrip_project_style() {
    // On écrit un fichier à partir de l'ASM, puis on relit et on re-désassemble.
    let assembled = asm::assemble(sample_asm()).expect("assemble ok");
    let path = temp_path("project_roundtrip.vitbc");

    loader::save_raw_program_to_path(&path, &assembled.program, false).expect("save ok");
    let re = loader::load_raw_program_from_path(&path).expect("load ok");

    // Re-désassemble et compare quelques invariants
    let dis = asm::disassemble(&re, &OpcodeTable::new_default());
    assert!(dis.contains("LOADI r0, 42"));
    assert!(dis.contains("RET"));

    let _ = fs::remove_file(&path);
}

#[test]
fn manual_raw_program_build_and_load() {
    // Construction manuelle d’un RawProgram minimaliste
    let mut prog = RawProgram::default();
    prog.entry_pc = Some(1);
    // Une constante et une string
    prog.const_pool.ints.insert("X".into(), 7);
    prog.const_pool.strings.insert("msg".into(), "OK".into());
    // Un petit DATA avec adresse
    prog.data_blobs.push(vitte_vm::asm::DataBlob {
        name: Some("blk".into()),
        bytes: vec![9, 9, 9],
        addr: Some(0x2000),
    });
    // Code : LOADI r0, 1 ; RET
    prog.code.push(RawOp { opcode: 0x03, argc: 2, args: [0, 1, 0] });
    prog.code.push(RawOp { opcode: 0x0C, argc: 0, args: [0, 0, 0] });

    // Sauvegarde et relecture
    let mut buf = Vec::new();
    loader::save_raw_program(&mut buf, &prog, false).expect("save ok");
    let back = loader::load_raw_program(&buf[..]).expect("load ok");

    assert_eq!(back.entry_pc, Some(1));
    assert_eq!(back.code.len(), 2);
    assert_eq!(back.const_pool.ints.get("X"), Some(&7));
    assert_eq!(back.const_pool.strings.get("msg").map(String::as_str), Some("OK"));
}

#[test]
fn disassemble_readable_for_floats_and_mem_operands() {
    // Petite source avec flottants et opérandes mémoire
    let src = r#"
        .entry start
    start:
        LOADF r0, 3.5
        LOADM r1, [r7, -16]
        STORM [r7, 32], r2
        RET
    "#;
    let out = asm::assemble(src).expect("assemble ok");
    let text = asm::disassemble(&out.program, &OpcodeTable::new_default());

    assert!(text.contains("LOADF r0, 3.5"));
    assert!(text.contains("[r7, -16]"));
    assert!(text.contains("[r7, 32]"));
    assert!(text.contains("RET"));
}
