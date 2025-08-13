#!/usr/bin/env bash
# gen_bytecode.sh — Génère un fichier VITBC (bytecode Vitte)
# ----------------------------------------------------------
# Deux modes :
#  1) Mode "chunk" (par défaut) : écrit un petit bytecode qui affiche un message.
#  2) Mode "asm" (--asm <src.asm>) : assemble la source texte → VITBC.
#
# Exemples :
#   ./scripts/gen_bytecode.sh                                  # ./hello.vitbc (chunk, msg par défaut)
#   ./scripts/gen_bytecode.sh -o out.vitbc -m "Salut Vitte!"   # chunk avec message custom
#   ./scripts/gen_bytecode.sh --asm examples/demo.asm -o demo.vitbc
#   ./scripts/gen_bytecode.sh --asm demo.asm -o demo.vitbc --compress
#
# Options :
#   -o, --out <PATH>       Chemin de sortie (.vitbc) [défaut: ./hello.vitbc]
#   -m, --msg <TXT>        Message pour le mode "chunk" [défaut: "Hello, Vitte!"]
#       --asm <FILE>       Fichier source ASM à assembler (active le mode ASM)
#       --compress         Sauvegarde compressée (zstd) — nécessite la feature zstd de vitte-vm
#       --level <N>        (réservé) niveau zstd (géré par vitte-vm; sinon ignoré)
#       --keep-tmp         Ne pas supprimer le dossier temporaire (debug)
#   -h, --help             Affiche l’aide
#
# Prérequis : cargo + rustc. Les dossiers "vitte-core" et "vitte-vm" doivent
# exister à la racine du repo (scripts/ est au même niveau).

set -Eeuo pipefail

# --- Couleurs terminal ---
if [[ -t 1 ]]; then
  BOLD=$'\033[1m'; DIM=$'\033[2m'; RED=$'\033[31m'; GRN=$'\033[32m'; YEL=$'\033[33m'; CYA=$'\033[36m'; RST=$'\033[0m'
else
  BOLD=""; DIM=""; RED=""; GRN=""; YEL=""; CYA=""; RST=""
fi
die() { echo "${RED}✖${RST} $*" >&2; exit 1; }
ok()  { echo "${GRN}✔${RST} $*"; }
note(){ echo "${CYA}ℹ${RST} $*"; }

# --- Aide ---
usage() {
  sed -n '1,120p' "$0" | sed -n '1,120p'
  exit 0
}

OUT="./hello.vitbc"
MSG="Hello, Vitte!"
ASM=""
COMPRESS=0
LEVEL="10"
KEEP_TMP=0

# --- Parse args ---
while (( $# )); do
  case "$1" in
    -h|--help) usage ;;
    -o|--out) shift; OUT="${1:-}"; [[ -z "$OUT" ]] && die "--out nécessite un chemin" ;;
    -m|--msg) shift; MSG="${1:-}";;
    --asm) shift; ASM="${1:-}"; [[ -z "$ASM" ]] && die "--asm nécessite un fichier";;
    --compress) COMPRESS=1 ;;
    --level) shift; LEVEL="${1:-10}" ;;
    --keep-tmp) KEEP_TMP=1 ;;
    *) die "Option inconnue: $1 (voir --help)";;
  esac
  shift
done

# --- Outils requis ---
command -v cargo >/dev/null 2>&1 || die "cargo introuvable. Installe rustup: https://rustup.rs/"
command -v rustc  >/dev/null 2>&1 || die "rustc introuvable."

# --- Racine repo (on remonte si git dispo) ---
ROOT="$(pwd)"
if command -v git >/dev/null 2>&1; then
  GROOT=$(git rev-parse --show-toplevel 2>/dev/null || true)
  [[ -n "$GROOT" ]] && ROOT="$GROOT"
fi
cd "$ROOT"

[[ -d "vitte-core" ]] || die "Dossier 'vitte-core' introuvable à la racine ($ROOT)"
[[ -d "vitte-vm"   ]] || die "Dossier 'vitte-vm' introuvable à la racine ($ROOT)"

# --- Préparation du projet éphémère ---
TMPDIR="$(mktemp -d -t vitte-gen-XXXXXX)"
trap '(( KEEP_TMP )) || rm -rf "$TMPDIR"' EXIT

note "Projet temporaire: ${DIM}$TMPDIR${RST}"

cat > "$TMPDIR/Cargo.toml" <<EOF
[package]
name = "vitte-gen"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
vitte-core = { path = "$ROOT/vitte-core" }
vitte-vm   = { path = "$ROOT/vitte-vm", features = ["zstd"] } # zstd optionnel côté vitte-vm

[[bin]]
name = "vitte-gen"
path = "src/main.rs"
EOF

mkdir -p "$TMPDIR/src"

# --- Programme Rust (génère soit depuis vitte-core, soit en assemblant) ---
cat > "$TMPDIR/src/main.rs" <<'RS'
use std::{env, fs, path::Path};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 3 {
        eprintln!("Usage (chunk): vitte-gen chunk <out.vitbc> <msg> <compress:0|1>");
        eprintln!("Usage (asm)  : vitte-gen asm <out.vitbc> <src.asm> <compress:0|1>");
        std::process::exit(2);
    }
    let mode = &args[0];
    if mode == "chunk" {
        let out = &args[1];
        let msg = &args[2];
        let compress = args.get(3).map(|s| s == "1").unwrap_or(false);

        // Génère un Chunk minimal avec vitte-core
        let mut c = vitte_core::helpers::new_chunk(false);
        let k = vitte_core::helpers::k_str(&mut c, msg);
        c.ops.push(vitte_core::Op::LoadConst(k));
        c.ops.push(vitte_core::Op::Print);
        c.ops.push(vitte_core::Op::Return);
        // Écrit bytes (format natif vitte-core)
        let bytes = c.to_bytes();
        fs::write(out, bytes).expect("write chunk");
        if compress {
            eprintln!("⚠ Compression ignorée en mode 'chunk' (format vitte-core, pas VITBC loader).");
        }
        println!("OK: écrit {out}");
    } else if mode == "asm" {
        let out = &args[1];
        let src = &args[2];
        let compress = args.get(3).map(|s| s == "1").unwrap_or(false);

        let asm = fs::read_to_string(src).expect("read asm");
        let assembled = vitte_vm::asm::assemble(&asm).expect("assemble");
        // Sauvegarde via loader VITBC v2 (compression selon flag)
        vitte_vm::loader::save_raw_program_to_path(out, &assembled.program, compress)
            .expect("save vitbc");
        println!("OK: écrit {out} (VITBC v2, compress={})", compress);
    } else {
        eprintln!("Mode inconnu: {mode}");
        std::process::exit(2);
    }
}
RS

# --- Build & run ---
pushd "$TMPDIR" >/dev/null

if [[ -n "$ASM" ]]; then
  [[ -f "$ASM" ]] || die "Fichier ASM introuvable: $ASM"
  note "Assemblage ASM → ${BOLD}$OUT${RST} (compress=${COMPRESS})"
  cargo run --quiet --bin vitte-gen -- asm "$OUT" "$ASM" "$COMPRESS"
else
  note "Génération chunk → ${BOLD}$OUT${RST}"
  cargo run --quiet --bin vitte-gen -- chunk "$OUT" "$MSG" "$COMPRESS"
fi

popd >/dev/null

ok "Fini. Fichier écrit : ${BOLD}$OUT${RST}"
(( KEEP_TMP )) && note "Dossier conservé : $TMPDIR"
