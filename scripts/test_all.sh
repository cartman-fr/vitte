#!/usr/bin/env bash
# test_all.sh — Orchestrateur de tests pour le workspace Vitte
# ------------------------------------------------------------
# Exemples :
#   ./scripts/test_all.sh                         # unit + intégration, debug, host
#   ./scripts/test_all.sh --release               # en release
#   ./scripts/test_all.sh --features "zstd"       # avec features
#   ./scripts/test_all.sh --all-features          # YOLO
#   ./scripts/test_all.sh --package vitte-vm      # un package précis
#   ./scripts/test_all.sh --integration-only      # ne lance que /tests (crate d’intégration)
#   ./scripts/test_all.sh --unit-only             # ne lance que les tests unitaires workspace
#   ./scripts/test_all.sh --coverage              # rapport coverage (html + lcov)
#   ./scripts/test_all.sh --nextest --retry 2     # nextest + retries
#   ./scripts/test_all.sh --ignored               # exécute aussi les #[ignore]
#
# Options :
#   -r, --release
#   -f, --features "<a,b>"
#       --all-features | --no-default-features
#   -p, --package <name> | --workspace
#       --target <triple>    (ex: x86_64-unknown-linux-gnu)
#       --jobs <N>
#       --locked | --frozen
#       --unit-only | --integration-only
#       --nextest            (utilise cargo-nextest si dispo)
#       --retry <N>          (avec nextest)
#       --ignored            (inclure #[ignore])
#       --nocapture          (affiche stdout des tests cargo test)
#       --coverage           (via cargo-llvm-cov)
#       --open               (ouvre le rapport HTML si --coverage)
#       --quiet | --verbose
#   -h, --help
#
# Sort avec code ≠ 0 au moindre pépin. On met RUST_BACKTRACE=1 pour la bagarre.

set -Eeuo pipefail

# ---------- UI ----------
if [[ -t 1 ]]; then
  B=$'\033[1m'; D=$'\033[2m'; R=$'\033[31m'; G=$'\033[32m'; Y=$'\033[33m'; C=$'\033[36m'; Z=$'\033[0m'
else
  B=""; D=""; R=""; G=""; Y=""; C=""; Z=""
fi
die(){ echo "${R}✖${Z} $*" >&2; exit 1; }
ok(){  echo "${G}✔${Z} $*"; }
note(){ echo "${C}ℹ${Z} $*"; }

usage(){ sed -n '1,120p' "$0" | sed -n '1,120p'; exit 0; }

# ---------- Defaults ----------
export RUST_BACKTRACE=1

RELEASE=0
FEATURES=""
ALL_FEATURES=0
NO_DEFAULT=0
PACKAGE=""
WORKSPACE=1
TARGET=""
JOBS=""
LOCKED=""
FROZEN=""
UNIT_ONLY=0
INTEG_ONLY=0
USE_NEXTEST=0
RETRY=0
INCLUDE_IGNORED=0
NOCAPTURE=0
COVERAGE=0
OPEN=0
QUIET=0
VERBOSE=0

# ---------- Parse args ----------
while (( $# )); do
  case "$1" in
    -r|--release) RELEASE=1 ;;
    -f|--features) shift; FEATURES="${1:-}";;
    --all-features) ALL_FEATURES=1 ;;
    --no-default-features) NO_DEFAULT=1 ;;
    -p|--package) shift; PACKAGE="${1:-}"; WORKSPACE=0 ;;
    --workspace) WORKSPACE=1 ;;
    --target) shift; TARGET="${1:-}";;
    --jobs) shift; JOBS="${1:-}";;
    --locked) LOCKED="--locked" ;;
    --frozen) FROZEN="--frozen" ;;
    --unit-only) UNIT_ONLY=1 ;;
    --integration-only) INTEG_ONLY=1 ;;
    --nextest) USE_NEXTEST=1 ;;
    --retry) shift; RETRY="${1:-0}" ;;
    --ignored) INCLUDE_IGNORED=1 ;;
    --nocapture) NOCAPTURE=1 ;;
    --coverage) COVERAGE=1 ;;
    --open) OPEN=1 ;;
    --quiet) QUIET=1 ;;
    --verbose) VERBOSE=1 ;;
    -h|--help) usage ;;
    *) die "Option inconnue: $1 (voir --help)" ;;
  esac
  shift
done

# Sanity
if (( UNIT_ONLY && INTEG_ONLY )); then die "--unit-only et --integration-only sont exclusifs"; fi

# ---------- Tools ----------
command -v cargo >/dev/null 2>&1 || die "cargo introuvable (installe rustup)."
command -v rustc >/dev/null 2>&1 || die "rustc introuvable."

HAVE_NEXTEST=0
if command -v cargo-nextest >/dev/null 2>&1; then HAVE_NEXTEST=1; fi
if (( USE_NEXTEST && !HAVE_NEXTEST )); then note "nextest demandé mais non installé → fallback cargo test"; fi

HAVE_LLVM_COV=0
if command -v cargo-llvm-cov >/dev/null 2>&1; then HAVE_LLVM_COV=1; fi
if (( COVERAGE && !HAVE_LLVM_COV )); then die "--coverage requis 'cargo-llvm-cov' (cargo install cargo-llvm-cov)"; fi

# ---------- Flags ----------
CARGO_FLAGS=()
(( QUIET )) && CARGO_FLAGS+=("--quiet")
(( VERBOSE )) && CARGO_FLAGS+=("-v")
[[ -n "$LOCKED" ]] && CARGO_FLAGS+=("$LOCKED")
[[ -n "$FROZEN" ]] && CARGO_FLAGS+=("$FROZEN")
[[ -n "$TARGET" ]] && CARGO_FLAGS+=("--target" "$TARGET")
[[ -n "$JOBS" ]] && CARGO_FLAGS+=("-j" "$JOBS")

FEATURE_FLAGS=()
if (( ALL_FEATURES )); then
  FEATURE_FLAGS+=("--all-features")
else
  (( NO_DEFAULT )) && FEATURE_FLAGS+=("--no-default-features")
  [[ -n "$FEATURES" ]] && FEATURE_FLAGS+=("--features" "$FEATURES")
fi

SCOPE_FLAGS=()
if (( WORKSPACE )); then
  SCOPE_FLAGS+=("--workspace")
elif [[ -n "$PACKAGE" ]]; then
  SCOPE_FLAGS+=("-p" "$PACKAGE")
fi

PROFILE_FLAGS=()
(( RELEASE )) && PROFILE_FLAGS+=("--release")

TEST_RUNNER="cargo test"
if (( USE_NEXTEST && HAVE_NEXTEST )); then
  TEST_RUNNER="cargo nextest run"
fi

# ---------- Helpers ----------
run_tests_workspace() {
  local extra=("$@")
  if [[ "$TEST_RUNNER" == "cargo nextest run" ]]; then
    local nx=()
    (( RETRY > 0 )) && nx+=("--retries" "$RETRY")
    (( INCLUDE_IGNORED )) && nx+=("--include-ignored")
    note "Tests (nextest)…"
    $TEST_RUNNER "${SCOPE_FLAGS[@]}" "${CARGO_FLAGS[@]}" "${FEATURE_FLAGS[@]}" "${PROFILE_FLAGS[@]}" "${extra[@]}" "${nx[@]}"
  else
    local ct=()
    (( INCLUDE_IGNORED )) && ct+=("--ignored")
    (( NOCAPTURE )) && ct+=("--nocapture")
    note "Tests (cargo test)…"
    cargo test "${SCOPE_FLAGS[@]}" "${CARGO_FLAGS[@]}" "${FEATURE_FLAGS[@]}" "${PROFILE_FLAGS[@]}" "${extra[@]}" "${ct[@]}"
  fi
}

run_tests_integration_crate() {
  # On table sur le crate /tests nommé vitte-vm-tests (comme tu l’as configuré)
  local extras=("$@")
  if [[ "$TEST_RUNNER" == "cargo nextest run" ]]; then
    local nx=()
    (( RETRY > 0 )) && nx+=("--retries" "$RETRY")
    (( INCLUDE_IGNORED )) && nx+=("--include-ignored")
    note "Intégration (/tests)…"
    cargo nextest run -p vitte-vm-tests "${CARGO_FLAGS[@]}" "${FEATURE_FLAGS[@]}" "${PROFILE_FLAGS[@]}" "${extras[@]}" "${nx[@]}"
  else
    local ct=()
    (( INCLUDE_IGNORED )) && ct+=("--ignored")
    (( NOCAPTURE )) && ct+=("--nocapture")
    note "Intégration (/tests)…"
    cargo test -p vitte-vm-tests "${CARGO_FLAGS[@]}" "${FEATURE_FLAGS[@]}" "${PROFILE_FLAGS[@]}" "${extras[@]}" "${ct[@]}"
  fi
}

# ---------- Coverage ----------
if (( COVERAGE )); then
  note "Couverture (cargo-llvm-cov) — préparation…"
  cargo llvm-cov clean --workspace || true
  # On calcule la couverture sur tout : unit + integration + doctests
  # Le runner interne s’occupe d’agréger.
  local_feat=("${FEATURE_FLAGS[@]}")
  local_scope=("--workspace") # couvrir tout le monde
  local_prof=()
  (( RELEASE )) && local_prof+=("--release")
  # On fournit la même cible/flags
  local_cargo=("${CARGO_FLAGS[@]}")

  note "Couverture en cours (ça peut être un peu long)…"
  cargo llvm-cov --no-report "${local_scope[@]}" "${local_feat[@]}" "${local_prof[@]}" "${local_cargo[@]}" \
    ${TARGET:+--target "$TARGET"} \
    ${ALL_FEATURES:+--all-features} \
    ${NO_DEFAULT:+--no-default-features} \
    --doctests --tests

  mkdir -p coverage
  cargo llvm-cov report --html --output-path coverage/html
  cargo llvm-cov report --lcov --output-path coverage/lcov.info
  ok "Coverage → ${B}coverage/html/index.html${Z} & ${B}coverage/lcov.info${Z}"
  if (( OPEN )); then
    if command -v xdg-open >/dev/null 2>&1; then xdg-open coverage/html/index.html || true
    elif command -v open >/dev/null 2>&1; then open coverage/html/index.html || true
    fi
  fi
  exit 0
fi

# ---------- Exécution tests ----------
if (( INTEG_ONLY )); then
  run_tests_integration_crate
  ok "Intégration OK"
  exit 0
fi

if (( UNIT_ONLY )); then
  run_tests_workspace
  ok "Unitaires OK"
  exit 0
fi

# Par défaut : unitaires workspace + intégration
run_tests_workspace
ok "Unitaires OK"
run_tests_integration_crate
ok "Intégration OK"

echo
ok "Tous les tests sont verts 🌱"
