#!/usr/bin/env bash
# release_package.sh — Prépare des artefacts de release multi-cibles pour Vitte
# -----------------------------------------------------------------------------
# Exemples :
#   ./scripts/release_package.sh
#   ./scripts/release_package.sh --version 0.2.0 --features "zstd" --targets "x86_64-unknown-linux-gnu,aarch64-apple-darwin"
#   ./scripts/release_package.sh --bins "vitte-vm,vitte-asm,disasm" --sign --gpg-key ABCDEF...
#   CROSS=1 ./scripts/release_package.sh --targets "x86_64-unknown-linux-musl"
#
# Options :
#   --version <X.Y.Z>           Force la version (sinon auto depuis vitte-core/Cargo.toml ou git tag)
#   --bins "a,b,c"              Liste des binaires à empaqueter (défaut: "vitte-vm,vitte-asm,disasm")
#   --targets "t1,t2"           Triples cibles (défaut: linux x86_64/aarch64, mac x86_64/arm64, win x86_64)
#   --features "<list>"         Chaîne de features à activer (ex: "zstd,serde")
#   --all-features              Active toutes les features
#   --no-default-features       Désactive les features par défaut
#   --locked / --frozen         Respect strict du Cargo.lock
#   --sign                      Signe les archives et le fichier SHASUMS (GPG détaché)
#   --gpg-key <KEYID>           Clé GPG à utiliser
#   --output-dir <dir>          Dossier de sortie (défaut: release/)
#   --dry-run                   N’exécute rien, affiche seulement
#   --keep-unpacked             Conserve les dossiers non archivé (debug/inspection)
#   -v | --verbose              Plus verbeux
#   -h | --help                 Aide
#
# Variables :
#   CROSS=1                     Utilise `cross` au lieu de `cargo` si installé
#
# Notes :
#   - On met tous les binaires choisis dans **une archive par target** (bundle).
#   - On strip les binaires si possible (strip/llvm-strip).
#   - Archives nommées : vitte-${VERSION}-${TARGET}.{tar.gz|zip}
#   - SHA256 dans release/SHASUMS256.txt (+ .sig si --sign)
# -----------------------------------------------------------------------------

set -Eeuo pipefail

# ---- UI ----
if [[ -t 1 ]]; then
  B=$'\033[1m'; D=$'\033[2m'; R=$'\033[31m'; G=$'\033[32m'; Y=$'\033[33m'; C=$'\033[36m'; Z=$'\033[0m'
else
  B=""; D=""; R=""; G=""; Y=""; C=""; Z=""
fi
die(){ echo "${R}✖${Z} $*" >&2; exit 1; }
ok(){  echo "${G}✔${Z} $*"; }
note(){ echo "${C}ℹ${Z} $*"; }
warn(){ echo "${Y}⚠${Z} $*"; }

# ---- Defaults ----
VERSION=""
BINS="vitte-vm,vitte-asm,disasm"
TARGETS="x86_64-unknown-linux-gnu,aarch64-unknown-linux-gnu,x86_64-apple-darwin,aarch64-apple-darwin,x86_64-pc-windows-msvc"
FEATURES=""
ALL_FEATURES=0
NO_DEFAULT_FEATURES=0
LOCKED=""
FROZEN=""
SIGN=0
GPG_KEY=""
OUTDIR="release"
DRY=0
KEEP_UNPACKED=0
VERBOSE=0

# ---- Args ----
while (( $# )); do
  case "$1" in
    --version) shift; VERSION="${1:-}";;
    --bins) shift; BINS="${1:-}";;
    --targets) shift; TARGETS="${1:-}";;
    --features) shift; FEATURES="${1:-}";;
    --all-features) ALL_FEATURES=1;;
    --no-default-features) NO_DEFAULT_FEATURES=1;;
    --locked) LOCKED="--locked";;
    --frozen) FROZEN="--frozen";;
    --sign) SIGN=1;;
    --gpg-key) shift; GPG_KEY="${1:-}";;
    --output-dir) shift; OUTDIR="${1:-}";;
    --dry-run) DRY=1;;
    --keep-unpacked) KEEP_UNPACKED=1;;
    -v|--verbose) VERBOSE=1;;
    -h|--help)
      grep -E '^#( |$)|^#!/' -n "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    *) die "Option inconnue: $1 (voir --help)";;
  esac
  shift
done

# ---- Tools ----
CARGO="cargo"
if [[ "${CROSS:-0}" != "0" ]]; then
  if command -v cross >/dev/null 2>&1; then CARGO="cross"; else warn "CROSS=1 mais 'cross' introuvable → fallback cargo"; fi
fi
command -v cargo >/dev/null 2>&1 || die "cargo introuvable"
command -v git >/dev/null 2>&1 || warn "git introuvable (ok)"

# ---- Repo root ----
ROOT="$(pwd)"
if git rev-parse --show-toplevel >/dev/null 2>&1; then ROOT="$(git rev-parse --show-toplevel)"; fi
cd "$ROOT"

# ---- Version auto si non fournie ----
if [[ -z "$VERSION" ]]; then
  if [[ -f "vitte-core/Cargo.toml" ]]; then
    VERSION="$(grep -m1 '^version\s*=' vitte-core/Cargo.toml | head -n1 | sed -E 's/.*"([^"]+)".*/\1/')"
  fi
  if [[ -z "$VERSION" && -n "$(git tag --list 'v*' | tail -n1)" ]]; then
    VERSION="$(git tag --list 'v*' | sort -V | tail -n1 | sed 's/^v//')"
  fi
fi
[[ -n "$VERSION" ]] || die "Impossible de déterminer la version (passe --version)"

note "Version : ${B}${VERSION}${Z}"
note "Cibles  : ${D}${TARGETS}${Z}"
note "Binaires: ${D}${BINS}${Z}"

# ---- Split CSV ----
IFS=',' read -r -a BIN_LIST <<< "$BINS"
IFS=',' read -r -a TGT_LIST <<< "$TARGETS"

# ---- Feature flags ----
FEATURE_FLAGS=()
if (( ALL_FEATURES )); then FEATURE_FLAGS+=(--all-features); else
  (( NO_DEFAULT_FEATURES )) && FEATURE_FLAGS+=(--no-default-features)
  [[ -n "$FEATURES" ]] && FEATURE_FLAGS+=(--features "$FEATURES")
fi

CARGO_BASE_FLAGS=()
[[ -n "$LOCKED" ]] && CARGO_BASE_FLAGS+=("$LOCKED")
[[ -n "$FROZEN" ]] && CARGO_BASE_FLAGS+=("$FROZEN")
(( VERBOSE )) && CARGO_BASE_FLAGS+=("-v")

# ---- Prepare out dir ----
mkdir -p "$OUTDIR"
SHAFILE="$OUTDIR/SHASUMS256.txt"
: > "$SHAFILE"

# ---- Helpers ----
find_bin_path() {
  local tgt="$1" bin="$2"
  local exe="$bin"
  [[ "$tgt" == *"-pc-windows-"* ]] && exe="${bin}.exe"
  echo "target/${tgt}/release/${exe}"
}

do() {
  if (( DRY )); then echo "DRY: $*"; else "$@"; fi
}

strip_bin() {
  local f="$1" tgt="$2"
  [[ ! -f "$f" ]] && return 0
  if [[ "$tgt" == *"-apple-darwin" ]]; then
    command -v strip >/dev/null 2>&1 && do strip -x "$f" || true
  elif [[ "$tgt" == *"-pc-windows-"* ]]; then
    # Pas de strip fiable en général sous Windows CI → on skip
    true
  else
    if command -v llvm-strip >/dev/null 2>&1; then
      do llvm-strip -s "$f" || true
    elif command -v strip >/dev/null 2>&1; then
      do strip -s "$f" || true
    fi
  fi
}

archive_name() {
  local tgt="$1"
  local ext="tar.gz"
  [[ "$tgt" == *"-pc-windows-"* ]] && ext="zip"
  echo "vitte-${VERSION}-${tgt}.${ext}"
}

mk_archive() {
  local tgt="$1" dir="$2" outpath="$3"
  (cd "$dir"
    if [[ "$outpath" == *.zip ]]; then
      if command -v zip >/dev/null 2>&1; then
        do zip -rq "../$outpath" .
      else
        die "zip introuvable pour créer $outpath"
      fi
    else
      do tar czf "../$outpath" .
    fi
  )
}

checksum() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" >> "$SHAFILE"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" >> "$SHAFILE"
  else
    die "sha256sum/shasum introuvable"
  fi
}

maybe_sign() {
  local file="$1"
  (( SIGN )) || return 0
  command -v gpg >/dev/null 2>&1 || die "gpg requis pour --sign"
  local args=(--armor --detach-sign)
  [[ -n "$GPG_KEY" ]] && args+=(--local-user "$GPG_KEY")
  do gpg "${args[@]}" "$file"
}

# ---- Files additionnels ----
COPY_FILES=()
for f in README.md README LICENSE LICENSE.* COPYING CHANGELOG.md CHANGELOG; do
  [[ -e "$f" ]] && COPY_FILES+=("$f")
done

# ---- Build par target ----
for tgt in "${TGT_LIST[@]}"; do
  echo
  note "▶ Build ${B}${tgt}${Z}"

  # Build release (workspace, car on empaquette plusieurs bins)
  if [[ "$CARGO" == "cross" ]]; then
    do cross build --workspace --release --target "$tgt" "${FEATURE_FLAGS[@]}" "${CARGO_BASE_FLAGS[@]}"
  else
    do cargo build --workspace --release --target "$tgt" "${FEATURE_FLAGS[@]}" "${CARGO_BASE_FLAGS[@]}"
  fi

  # Vérifie la présence des binaires demandés
  STAGE_DIR="${OUTDIR}/vitte-${VERSION}-${tgt}"
  do rm -rf "$STAGE_DIR"
  do mkdir -p "$STAGE_DIR"

  # Copie + strip
  for bin in "${BIN_LIST[@]}"; do
    [[ -z "$bin" ]] && continue
    BIN_PATH="$(find_bin_path "$tgt" "$bin")"
    if [[ ! -f "$BIN_PATH" ]]; then
      warn "Binaire manquant pour ${bin} @ ${tgt} → skip"
      continue
    fi
    OUT_BIN="${STAGE_DIR}/$([[ "$tgt" == *"-pc-windows-"* ]] && echo "${bin}.exe" || echo "${bin}")"
    do cp "$BIN_PATH" "$OUT_BIN"
    strip_bin "$OUT_BIN" "$tgt"
    ok "binaire: $(basename "$OUT_BIN")"
  done

  # Fichiers accompagnement
  for f in "${COPY_FILES[@]}"; do
    do cp "$f" "$STAGE_DIR/"
  done

  # Archive
  ARCHIVE="$(archive_name "$tgt")"
  mk_archive "$tgt" "$STAGE_DIR" "${OUTDIR}/${ARCHIVE}"
  ok "archive: ${OUTDIR}/${ARCHIVE}"

  # SHA + signature
  checksum "${OUTDIR}/${ARCHIVE}"
  maybe_sign "${OUTDIR}/${ARCHIVE}"

  # Nettoyage staging
  (( KEEP_UNPACKED )) || do rm -rf "$STAGE_DIR"
done

# ---- SHASUMS signature ----
maybe_sign "$SHAFILE"

echo
ok "Fini. Artefacts dans ${B}${OUTDIR}${Z}"
echo "${D}$(ls -1 ${OUTDIR})${Z}"
