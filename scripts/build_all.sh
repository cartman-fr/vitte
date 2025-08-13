#!/usr/bin/env bash
# build_all.sh — Build complet du workspace Vitte
# ------------------------------------------------
# Usage rapide :
#   ./scripts/build_all.sh                 # build debug (workspace)
#   ./scripts/build_all.sh -r              # build release
#   ./scripts/build_all.sh -r -f zstd      # build release avec feature zstd
#   ./scripts/build_all.sh --clippy --fmt  # lint + format check + build
#   ./scripts/build_all.sh --tests         # compile aussi les tests (no-run)
#   ./scripts/build_all.sh --doc           # génère la doc
#
# Options :
#   -r | --release                 Build en release
#   -f | --features "<list>"       Chaîne de features (ex: "zstd,serde")
#        --all-features            Active toutes les features
#        --no-default-features     Désactive les features par défaut
#   -p | --package <name|glob>     Restreint à un package
#        --target <triple>         Triple cible (x86_64-unknown-linux-gnu, wasm32-unknown-unknown, …)
#        --jobs <N>                Nombre de jobs (par défaut: auto)
#        --locked | --frozen       Respect du Cargo.lock (CI)
#        --clean                   Cargo clean avant build
#        --workspace               Force le build du workspace complet (défaut si pas de -p)
#        --clippy                  Lance clippy (warnings en erreurs)
#        --fmt                     Vérifie le format (cargo fmt --check)
#        --tests                   Compile aussi les tests (no-run)
#        --benches                 Compile les benches
#        --examples                Compile les examples
#        --doc                     Génére la doc (no-deps)
#        --quiet                   Moins verbeux
#        --verbose                 Plus verbeux
#   -h | --help                    Aide
#
# Exits non-zero au moindre pépin.

set -Eeuo pipefail

# --- Couleurs (si terminal) ---
if [[ -t 1 ]]; then
  BOLD=$'\033[1m'; DIM=$'\033[2m'; RED=$'\033[31m'; GRN=$'\033[32m'; YEL=$'\033[33m'; BLU=$'\033[34m'; MAG=$'\033[35m'; CYA=$'\033[36m'; RST=$'\033[0m'
else
  BOLD=""; DIM=""; RED=""; GRN=""; YEL=""; BLU=""; MAG=""; CYA=""; RST=""
fi

die() { echo "${RED}✖${RST} $*" >&2; exit 1; }
note(){ echo "${CYA}ℹ${RST} $*"; }
ok()  { echo "${GRN}✔${RST} $*"; }
warn(){ echo "${YEL}⚠${RST} $*"; }

# --- Aide ---
usage() {
  sed -n '1,120p' "$0" | sed -n '1,80p' | grep -v '^set -Eeuo pipefail' || true
  exit 0
}

# --- Defaults ---
RELEASE=0
FEATURES=""
ALL_FEATURES=0
NO_DEFAULT_FEATURES=0
PACKAGE=""
TARGET=""
JOBS=""
LOCKED_FLAG=""
FROZEN_FLAG=""
CLEAN=0
WORKSPACE=1
DO_CLIPPY=0
DO_FMT=0
DO_TESTS=0
DO_BENCHES=0
DO_EXAMPLES=0
DO_DOC=0
QUIET=0
VERBOSE=0

# --- Parse args ---
while (( $# )); do
  case "$1" in
    -r|--release) RELEASE=1 ;;
    -f|--features) shift; FEATURES="${1:-}"; [[ -z "${FEATURES}" ]] && die "--features requiert une liste";;
    --all-features) ALL_FEATURES=1 ;;
    --no-default-features) NO_DEFAULT_FEATURES=1 ;;
    -p|--package) shift; PACKAGE="${1:-}"; [[ -z "${PACKAGE}" ]] && die "--package requiert un nom"; WORKSPACE=0 ;;
    --target) shift; TARGET="${1:-}"; [[ -z "${TARGET}" ]] && die "--target requiert un triple";;
    --jobs) shift; JOBS="${1:-}"; [[ -z "${JOBS}" ]] && die "--jobs requiert un nombre";;
    --locked) LOCKED_FLAG="--locked" ;;
    --frozen) FROZEN_FLAG="--frozen" ;;
    --clean) CLEAN=1 ;;
    --workspace) WORKSPACE=1 ;;
    --clippy) DO_CLIPPY=1 ;;
    --fmt) DO_FMT=1 ;;
    --tests) DO_TESTS=1 ;;
    --benches) DO_BENCHES=1 ;;
    --examples) DO_EXAMPLES=1 ;;
    --doc) DO_DOC=1 ;;
    --quiet) QUIET=1 ;;
    --verbose) VERBOSE=1 ;;
    -h|--help) usage ;;
    *) die "Option inconnue: $1 (voir --help)";;
  esac
  shift
done

# --- Préchecks outils ---
command -v cargo >/dev/null 2>&1 || die "cargo introuvable. Installe Rust (https://rustup.rs/)"
command -v rustc  >/dev/null 2>&1 || die "rustc introuvable."

# --- Construit la ligne d’options Cargo ---
CARGO_FLAGS=()
(( QUIET ))   && CARGO_FLAGS+=("--quiet")
(( VERBOSE )) && CARGO_FLAGS+=("-v")
[[ -n "${LOCKED_FLAG}" ]] && CARGO_FLAGS+=("${LOCKED_FLAG}")
[[ -n "${FROZEN_FLAG}" ]] && CARGO_FLAGS+=("${FROZEN_FLAG}")
[[ -n "${TARGET}" ]] && CARGO_FLAGS+=("--target" "${TARGET}")
[[ -n "${JOBS}" ]] && CARGO_FLAGS+=("-j" "${JOBS}")

FEATURE_FLAGS=()
if (( ALL_FEATURES )); then
  FEATURE_FLAGS+=("--all-features")
else
  (( NO_DEFAULT_FEATURES )) && FEATURE_FLAGS+=("--no-default-features")
  [[ -n "${FEATURES}" ]] && FEATURE_FLAGS+=("--features" "${FEATURES}")
fi

SCOPE_FLAGS=()
if (( WORKSPACE )); then
  SCOPE_FLAGS+=("--workspace")
elif [[ -n "${PACKAGE}" ]]; then
  SCOPE_FLAGS+=("-p" "${PACKAGE}")
fi

PROFILE_FLAGS=()
(( RELEASE )) && PROFILE_FLAGS+=("--release")

# --- Clean optionnel ---
if (( CLEAN )); then
  note "Nettoyage (cargo clean)…"
  cargo clean "${SCOPE_FLAGS[@]}" || die "clean a échoué"
fi

# --- Fmt check optionnel ---
if (( DO_FMT )); then
  note "Vérification du format (cargo fmt --all --check)…"
  if ! cargo fmt --all -- --check; then
    warn "Formatage incorrect. Pour corriger : ${BOLD}cargo fmt --all${RST}"
    exit 2
  fi
  ok "Format OK"
fi

# --- Clippy optionnel ---
if (( DO_CLIPPY )); then
  note "Clippy (warnings = erreurs)…"
  cargo clippy "${SCOPE_FLAGS[@]}" "${CARGO_FLAGS[@]}" "${FEATURE_FLAGS[@]}" -- -D warnings || die "clippy a échoué"
  ok "Clippy OK"
fi

# --- Build debug/release ---
BUILD_KIND=$([[ ${RELEASE} -eq 1 ]] && echo "release" || echo "debug")
note "Build ${BOLD}${BUILD_KIND}${RST}…"
cargo build "${SCOPE_FLAGS[@]}" "${CARGO_FLAGS[@]}" "${FEATURE_FLAGS[@]}" "${PROFILE_FLAGS[@]}" || die "build ${BUILD_KIND} a échoué"
ok "Build ${BUILD_KIND} terminé"

# --- Compile tests (no-run) ---
if (( DO_TESTS )); then
  note "Compilation des tests (no-run)…"
  cargo test "${SCOPE_FLAGS[@]}" "${CARGO_FLAGS[@]}" "${FEATURE_FLAGS[@]}" "${PROFILE_FLAGS[@]}" --no-run || die "compilation des tests a échoué"
  ok "Tests compilés"
fi

# --- Compile benches ---
if (( DO_BENCHES )); then
  note "Compilation des benchmarks…"
  cargo bench "${SCOPE_FLAGS[@]}" "${CARGO_FLAGS[@]}" "${FEATURE_FLAGS[@]}" "${PROFILE_FLAGS[@]}" --no-run || die "compilation des benches a échoué"
  ok "Benches compilés"
fi

# --- Compile examples ---
if (( DO_EXAMPLES )); then
  note "Compilation des examples…"
  cargo build "${SCOPE_FLAGS[@]}" "${CARGO_FLAGS[@]}" "${FEATURE_FLAGS[@]}" "${PROFILE_FLAGS[@]}" --examples || die "compilation des examples a échoué"
  ok "Examples compilés"
fi

# --- Doc ---
if (( DO_DOC )); then
  note "Génération de la documentation…"
  cargo doc "${SCOPE_FLAGS[@]}" "${CARGO_FLAGS[@]}" "${FEATURE_FLAGS[@]}" --no-deps || die "doc a échoué"
  ok "Doc générée → target/doc"
fi

# --- Résumé ---
echo
ok "Tout est bon ✅"
echo "${DIM}Profil : ${BUILD_KIND} | Workspace : $([[ ${WORKSPACE} -eq 1 ]] && echo oui || echo non) | Features : ${FEATURES:-<par défaut>}${RST}"
[[ -n "${TARGET}" ]] && echo "${DIM}Cible : ${TARGET}${RST}"

exit 0
