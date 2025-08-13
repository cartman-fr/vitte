#!/usr/bin/env bash
# ci_check.sh — Orchestrateur CI pour le workspace Vitte
# ------------------------------------------------------
# Objectif : un pipeline reproductible, bavard quand il faut, strict
# quand on lui demande. Fonctionne en local comme en CI.
#
# Exemples :
#   ./scripts/ci_check.sh
#   ./scripts/ci_check.sh --strict --features "zstd,serde" --targets "x86_64-unknown-linux-gnu,wasm32-unknown-unknown"
#   ./scripts/ci_check.sh --fast    # saute audit/doc/benches
#
# Variables d’environnement utiles :
#   CI_FAST=1                # équivalent --fast
#   CI_STRICT=1              # équivalent --strict (warnings => erreurs)
#   CI_FEATURES="..."        # liste de features à tester (csv)
#   CI_TARGETS="..."         # liste de cibles à builder (csv)
#   CI_PACKAGES="..."        # liste de packages à tester (csv); défaut: --workspace
#   CI_NO_DEFAULT_FEATURES=1 # construit sans features par défaut
#   CI_ALL_FEATURES=1        # teste avec toutes les features
#   RUSTFLAGS="..."          # flags rustc additionnels
#   SCCACHE_BUCKET=...       # si sccache est configuré
#
# Requirements optionnels (auto-détectés) :
#   cargo-nextest, cargo-hack, cargo-audit, cargo-deny
#
# Exit non-zero au moindre échec.

set -Eeuo pipefail

# ---------- Style ----------
if [[ -t 1 ]]; then
  BOLD=$'\033[1m'; DIM=$'\033[2m'; RED=$'\033[31m'; GRN=$'\033[32m'; YEL=$'\033[33m'; CYA=$'\033[36m'; RST=$'\033[0m'
else
  BOLD=""; DIM=""; RED=""; GRN=""; YEL=""; CYA=""; RST=""
fi
die() { echo "${RED}✖${RST} $*" >&2; exit 1; }
ok()  { echo "${GRN}✔${RST} $*"; }
note(){ echo "${CYA}ℹ${RST} $*"; }
warn(){ echo "${YEL}⚠${RST} $*"; }

# ---------- Options ----------
STRICT=${CI_STRICT:-0}
FAST=${CI_FAST:-0}
FEATURES_CSV=${CI_FEATURES:-""}
TARGETS_CSV=${CI_TARGETS:-""}
PACKAGES_CSV=${CI_PACKAGES:-""}
ALL_FEATURES=${CI_ALL_FEATURES:-0}
NO_DEFAULT_FEATURES=${CI_NO_DEFAULT_FEATURES:-0}
VERBOSE=0

while (( $# )); do
  case "$1" in
    --strict) STRICT=1 ;;
    --fast) FAST=1 ;;
    --features) shift; FEATURES_CSV="${1:-}";;
    --targets) shift; TARGETS_CSV="${1:-}";;
    --packages) shift; PACKAGES_CSV="${1:-}";;
    --all-features) ALL_FEATURES=1 ;;
    --no-default-features) NO_DEFAULT_FEATURES=1 ;;
    -v|--verbose) VERBOSE=1 ;;
    -h|--help)
      grep -E '^(# |#$|#!/)' -n "$0" | sed 's/^# \{0,1\}//'
      exit 0 ;;
    *) die "Option inconnue: $1" ;;
  esac
  shift
done

# ---------- Préchecks ----------
command -v cargo >/dev/null 2>&1 || die "cargo introuvable. Installe rustup (https://rustup.rs/)."
command -v rustc  >/dev/null 2>&1 || die "rustc introuvable."

CARGO_FLAGS=()
(( VERBOSE )) && CARGO_FLAGS+=("-v")

# Matrices
IFS=',' read -r -a MATRIX_FEATURES <<< "${FEATURES_CSV}"
IFS=',' read -r -a MATRIX_TARGETS  <<< "${TARGETS_CSV}"
IFS=',' read -r -a MATRIX_PACKAGES <<< "${PACKAGES_CSV}"

# Workspace / scope
SCOPE_FLAGS=()
if [[ -z "${PACKAGES_CSV}" ]]; then
  SCOPE_FLAGS+=(--workspace)
else
  for p in "${MATRIX_PACKAGES[@]}"; do
    [[ -n "$p" ]] && SCOPE_FLAGS+=(-p "$p")
  done
fi

# Features
FEATURE_FLAGS_BASE=()
if (( ALL_FEATURES )); then
  FEATURE_FLAGS_BASE+=(--all-features)
else
  (( NO_DEFAULT_FEATURES )) && FEATURE_FLAGS_BASE+=(--no-default-features)
fi

# ---------- Infos d’environnement ----------
note "Rust: $(rustc --version)"
note "Cargo: $(cargo --version)"
if command -v sccache >/dev/null 2>&1; then
  note "sccache: $(sccache --version)"
  export RUSTC_WRAPPER="$(command -v sccache)"
fi

# ---------- Étape: fmt ----------
note "Vérification du format (cargo fmt --check)…"
if ! cargo fmt --all -- --check; then
  warn "Formatage incorrect. Pour corriger : ${BOLD}cargo fmt --all${RST}"
  exit 2
fi
ok "Format OK"

# ---------- Étape: clippy ----------
CLIPPY_FLAGS=(-D warnings)
(( STRICT )) && CLIPPY_FLAGS+=(-W clippy::pedantic -W clippy::nursery)
note "Clippy (warnings → erreurs${STRICT:+, mode strict})…"
cargo clippy "${SCOPE_FLAGS[@]}" "${FEATURE_FLAGS_BASE[@]}" "${CARGO_FLAGS[@]}" -- "${CLIPPY_FLAGS[@]}" || die "clippy a échoué"
ok "Clippy OK"

# ---------- Helper: build/test/doc sur une combinaison (features, target, profile) ----------
build_combo() {
  local features="$1" target="$2" profile="$3" tests="$4" benches="$5" examples="$6" doc="$7"

  local feat_flags=("${FEATURE_FLAGS_BASE[@]}")
  [[ -n "$features" ]] && feat_flags+=(--features "$features")
  local tgt_flags=()
  [[ -n "$target" ]] && tgt_flags+=(--target "$target")

  note "Build ${profile} ${DIM}[features='${features:-<par défaut>}', target='${target:-host}']${RST}"
  if [[ "$profile" == "release" ]]; then
    cargo build "${SCOPE_FLAGS[@]}" "${feat_flags[@]}" "${tgt_flags[@]}" "${CARGO_FLAGS[@]}" --release
  else
    cargo build "${SCOPE_FLAGS[@]}" "${feat_flags[@]}" "${tgt_flags[@]}" "${CARGO_FLAGS[@]}"
  fi

  if [[ "$tests" == "1" ]]; then
    # Préfère nextest si dispo
    if command -v cargo-nextest >/dev/null 2>&1; then
      note "Tests (nextest)…"
      if [[ "$profile" == "release" ]]; then
        cargo nextest run "${SCOPE_FLAGS[@]}" "${feat_flags[@]}" "${tgt_flags[@]}" "${CARGO_FLAGS[@]}" --release
      else
        cargo nextest run "${SCOPE_FLAGS[@]}" "${feat_flags[@]}" "${tgt_flags[@]}" "${CARGO_FLAGS[@]}"
      fi
    else
      note "Tests (cargo test)…"
      if [[ "$profile" == "release" ]]; then
        cargo test "${SCOPE_FLAGS[@]}" "${feat_flags[@]}" "${tgt_flags[@]}" "${CARGO_FLAGS[@]}" --release
      else
        cargo test "${SCOPE_FLAGS[@]}" "${feat_flags[@]}" "${tgt_flags[@]}" "${CARGO_FLAGS[@]}"
      fi
    fi
  fi

  if [[ "$examples" == "1" ]]; then
    note "Compilation des examples…"
    if [[ "$profile" == "release" ]]; then
      cargo build "${SCOPE_FLAGS[@]}" "${feat_flags[@]}" "${tgt_flags[@]}" "${CARGO_FLAGS[@]}" --release --examples
    else
      cargo build "${SCOPE_FLAGS[@]}" "${feat_flags[@]}" "${tgt_flags[@]}" "${CARGO_FLAGS[@]}" --examples
    fi
  fi

  if [[ "$benches" == "1" ]]; then
    note "Compilation des benches (no-run)…"
    if [[ "$profile" == "release" ]]; then
      cargo bench "${SCOPE_FLAGS[@]}" "${feat_flags[@]}" "${tgt_flags[@]}" "${CARGO_FLAGS[@]}" --no-run --release
    else
      cargo bench "${SCOPE_FLAGS[@]}" "${feat_flags[@]}" "${tgt_flags[@]}" "${CARGO_FLAGS[@]}" --no-run
    fi
  fi

  if [[ "$doc" == "1" ]]; then
    note "Doc (no-deps)…"
    cargo doc "${SCOPE_FLAGS[@]}" "${feat_flags[@]}" "${tgt_flags[@]}" "${CARGO_FLAGS[@]}" --no-deps
  fi

  ok "Combo OK (${profile}, features='${features:-<par défaut>}', target='${target:-host}')"
}

# ---------- Étape: powerset features (optionnel) ----------
if command -v cargo-hack >/dev/null 2>&1; then
  if (( ! FAST )); then
    note "Exploration rapide du powerset de features (cargo hack)…"
    cargo hack check "${SCOPE_FLAGS[@]}" ${NO_DEFAULT_FEATURES:+--no-default-features} ${ALL_FEATURES:+--all-features} \
      --each-feature --no-dev-deps || die "cargo hack a échoué"
    ok "cargo hack OK"
  else
    note "Mode fast : skip cargo hack"
  fi
else
  note "cargo-hack non installé → skip powerset"
fi

# ---------- Matrices: cibles ----------
if [[ "${#MATRIX_TARGETS[@]}" -eq 0 || -z "${MATRIX_TARGETS[0]}" ]]; then
  MATRIX_TARGETS=("") # cible hôte
fi

# ---------- Matrices: features ----------
if [[ "${#MATRIX_FEATURES[@]}" -eq 0 || -z "${MATRIX_FEATURES[0]}" ]]; then
  MATRIX_FEATURES=("") # features par défaut
fi

# ---------- Build & test combos ----------
for tgt in "${MATRIX_TARGETS[@]}"; do
  for feats in "${MATRIX_FEATURES[@]}"; do
    # Debug complet (tests + examples, benches si pas fast)
    build_combo "$feats" "$tgt" "debug" "1" "$((FAST?0:1))" "1" "$((FAST?0:1))"
    # Release (tests uniquement, doc sautée en release par défaut)
    build_combo "$feats" "$tgt" "release" "1" "0" "0" "0"
  done
done

# ---------- Audit sécurité (optionnel) ----------
if (( ! FAST )); then
  if command -v cargo-audit >/dev/null 2>&1; then
    note "Audit sécurité (cargo-audit)…"
    cargo audit || die "cargo audit a détecté des vulnérabilités"
    ok "Audit OK"
  else
    note "cargo-audit non installé → skip"
  fi

  if command -v cargo-deny >/dev/null 2>&1; then
    note "Licence & bans (cargo-deny)…"
    cargo deny check || die "cargo deny a échoué"
    ok "Licences OK"
  else
    note "cargo-deny non installé → skip"
  fi
else
  note "Mode fast : skip audit/deny"
fi

# ---------- Résumé ----------
echo
ok "Pipeline CI local ✅"
echo "${DIM}Strict: ${STRICT} | Fast: ${FAST} | Features: '${FEATURES_CSV:-<par défaut>}' | Targets: '${TARGETS_CSV:-host}'${RST}"

exit 0
