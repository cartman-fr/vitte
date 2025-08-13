#!/usr/bin/env bash
# clean_all.sh — Nettoyage complet (mais intelligent) du workspace Vitte
# ---------------------------------------------------------------------
# Usages rapides :
#   ./scripts/clean_all.sh                 # cargo clean + purge des 'target/' et fichiers temporaires
#   ./scripts/clean_all.sh --deep          # + caches (coverage, docs, artefacts *.vitbc, *.vzbc)
#   ./scripts/clean_all.sh --sccache       # + purge cache sccache (si installé)
#   ./scripts/clean_all.sh --dry-run       # affiche ce qui serait supprimé
#   ./scripts/clean_all.sh -p vitte-vm     # ne clean que le package donné (cargo clean -p)
#
# Options :
#   -p, --package <name>          Package spécifique (sinon --workspace)
#       --workspace               Force le scope workspace
#       --target <triple>         Nettoie aussi target/<triple> s’il existe
#       --deep                    Nettoyage approfondi (docs, coverage, artefacts *.vitbc/*.vzbc)
#       --sccache                 Purge le cache sccache
#       --rm-lock                 Supprime Cargo.lock (⚠️ rare)
#       --yes                     Ne pas demander de confirmation
#       --dry-run                 N’affiche que les actions
#       --quiet / --verbose       Verbosité
#   -h, --help                    Aide
#
# Sort avec code non-nul au moindre pépin.

set -Eeuo pipefail

# --- Couleurs ---
if [[ -t 1 ]]; then
  BOLD=$'\033[1m'; DIM=$'\033[2m'; RED=$'\033[31m'; GRN=$'\033[32m'; YEL=$'\033[33m'; CYA=$'\033[36m'; RST=$'\033[0m'
else
  BOLD=""; DIM=""; RED=""; GRN=""; YEL=""; CYA=""; RST=""
fi
die() { echo "${RED}✖${RST} $*" >&2; exit 1; }
ok()  { echo "${GRN}✔${RST} $*"; }
note(){ echo "${CYA}ℹ${RST} $*"; }
warn(){ echo "${YEL}⚠${RST} $*"; }

usage() { sed -n '1,120p' "$0" | sed -n '1,100p'; exit 0; }

# --- Defaults ---
PKG=""
WORKSPACE=1
TARGET_TRIPLE=""
DEEP=0
SCCACHE=0
RM_LOCK=0
YES=0
DRY=0
QUIET=0
VERBOSE=0

# --- Parse args ---
while (( $# )); do
  case "$1" in
    -p|--package) shift; PKG="${1:-}"; [[ -z "$PKG" ]] && die "--package requiert un nom"; WORKSPACE=0 ;;
    --workspace) WORKSPACE=1 ;;
    --target) shift; TARGET_TRIPLE="${1:-}"; [[ -z "$TARGET_TRIPLE" ]] && die "--target requiert un triple" ;;
    --deep) DEEP=1 ;;
    --sccache) SCCACHE=1 ;;
    --rm-lock) RM_LOCK=1 ;;
    --yes) YES=1 ;;
    --dry-run) DRY=1 ;;
    --quiet) QUIET=1 ;;
    --verbose) VERBOSE=1 ;;
    -h|--help) usage ;;
    *) die "Option inconnue: $1 (voir --help)";;
  esac
  shift
done

# --- Contexte repo (on remonte à la racine si repo git) ---
if command -v git >/dev/null 2>&1; then
  ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "")
  [[ -n "$ROOT" ]] && cd "$ROOT"
fi

# --- Helpers de suppression ---
rm_path() {
  local p="$1"
  if [[ -e "$p" || -L "$p" ]]; then
    if (( DRY )); then
      [[ $QUIET -eq 0 ]] && echo "DRY: rm -rf '$p'"
    else
      rm -rf -- "$p" || warn "Échec suppression: $p"
      [[ $QUIET -eq 0 ]] && echo "rm -rf '$p'"
    fi
  fi
}
rm_glob() {
  local pat="$1"
  shopt -s nullglob dotglob
  local hits=($pat)
  shopt -u nullglob dotglob
  for f in "${hits[@]}"; do rm_path "$f"; done
}

confirm() {
  (( YES )) && return 0
  read -r -p "$(echo -e "${YEL}Confirmer le nettoyage ?${RST} [y/N] ")" ans
  [[ "${ans,,}" == "y" || "${ans,,}" == "yes" ]]
}

# --- Affiche contexte ---
[[ $QUIET -eq 0 ]] && note "Nettoyage${DEEP:+ (deep)} — repo: ${BOLD}$(pwd)${RST}"

# --- Étape 0 : Confirmation ---
confirm || { warn "Annulé."; exit 130; }

# --- Étape 1 : cargo clean ---
CLEAN_SCOPE=()
if (( WORKSPACE )); then
  CLEAN_SCOPE+=(--workspace)
elif [[ -n "$PKG" ]]; then
  CLEAN_SCOPE+=(-p "$PKG")
fi

if command -v cargo >/dev/null 2>&1; then
  if (( DRY )); then
    [[ $QUIET -eq 0 ]] && echo "DRY: cargo clean ${CLEAN_SCOPE[*]}"
  else
    cargo clean "${CLEAN_SCOPE[@]}" || warn "cargo clean a échoué"
  fi
else
  warn "cargo introuvable — on skip 'cargo clean'"
fi

# --- Étape 2 : purges de dossiers 'target/' (déplacés/anciens) ---
#   - racine ./target
#   - membres (si des 'target' parasites existent)
#   - cible spécifique target/<triple> si demandée
if [[ -d target ]]; then
  if [[ -n "$TARGET_TRIPLE" && -d "target/$TARGET_TRIPLE" ]]; then
    rm_path "target/$TARGET_TRIPLE"
  fi
  # On vire quand même target/ entier (cargo clean l'a déjà fait, mais parfois des restes restent)
  rm_path "target"
fi

# Supprime 'target' égarés (ex.: dans sous-projets déplacés)
while IFS= read -r -d '' dir; do
  rm_path "$dir"
done < <(find . -type d -name target -prune -print0 2>/dev/null)

# --- Étape 3 : artefacts temporaires généraux ---
rm_glob "**/*.swp"
rm_glob "**/*.swo"
rm_glob "**/*~"
rm_glob "**/.DS_Store"
rm_glob "**/.AppleDouble"
rm_glob "**/.Spotlight-V100"
rm_glob "**/.Trash*"

# --- Étape 4 : artefacts Vitte/bytecode & tests ---
# On n’efface **pas** sources. Seulement outputs.
rm_glob "**/*.vitbc"
rm_glob "**/*.vzbc"
rm_glob "tests/**/*.tmp"
rm_glob "tests/**/target"
rm_glob ".tmp"
rm_glob ".cache"

# --- Étape 5 : coverage & docs (deep only) ---
if (( DEEP )); then
  rm_glob "coverage"
  rm_glob "**/*.profraw"
  rm_glob "**/*.profdata"
  rm_glob "target/llvm-cov"
  rm_glob "target/kcov*"
  rm_glob "target/tarpaulin-report*"
  rm_glob "target/doc"
  rm_glob "docs/target"
fi

# --- Étape 6 : sccache (optionnel) ---
if (( SCCACHE )); then
  if command -v sccache >/dev/null 2>&1; then
    if (( DRY )); then
      [[ $QUIET -eq 0 ]] && echo "DRY: sccache --clear-cache && sccache --zero-stats"
    else
      sccache --clear-cache || warn "sccache --clear-cache a échoué"
      sccache --zero-stats || true
    fi
  else
    warn "sccache non installé — skip"
  fi
fi

# --- Étape 7 : Cargo.lock (optionnel, dangereux) ---
if (( RM_LOCK )); then
  if [[ -f Cargo.lock ]]; then
    rm_path "Cargo.lock"
  fi
fi

ok "Nettoyage terminé ${DIM}(dry-run=${DRY}, deep=${DEEP})${RST}"
exit 0
