#!/usr/bin/env bash
# gen_licenses.sh — Génère l’inventaire des licences (workspace Rust)
# -------------------------------------------------------------------
# Sorties par défaut (dans ./licenses) :
#   - THIRD-PARTY.csv              (toutes les deps transitives)
#   - THIRD-PARTY.json
#   - THIRD-PARTY-direct.csv       (deps directes uniquement)
#   - THIRD-PARTY-direct.json
#   - NOTICE-THIRD-PARTY.txt       (résumé humain + conseils)
#
# Exemples :
#   ./scripts/gen_licenses.sh
#   ./scripts/gen_licenses.sh --out licenses --deny-nonspdx
#   ./scripts/gen_licenses.sh --include-build-deps
#   ./scripts/gen_licenses.sh --use-cargo-deny
#
# Options :
#   --out <dir>                Dossier de sortie (défaut: licenses)
#   --include-build-deps       Inclure les build-deps (par défaut on les évite)
#   --deny-nonspdx             Sort en erreur si licences manquantes/“unknown”
#   --use-cargo-deny           Tente un `cargo deny check licenses` (si dispo)
#   --quiet / --verbose
#   -h | --help
#
# Tips d’install (si besoin) :
#   cargo install cargo-license
#   cargo install cargo-deny
#
set -Eeuo pipefail

# -------- UI --------
if [[ -t 1 ]]; then
  B=$'\033[1m'; D=$'\033[2m'; R=$'\033[31m'; G=$'\033[32m'; Y=$'\033[33m'; C=$'\033[36m'; Z=$'\033[0m'
else
  B=""; D=""; R=""; G=""; Y=""; C=""; Z=""
fi
die(){ echo "${R}✖${Z} $*" >&2; exit 2; }
ok(){  echo "${G}✔${Z} $*"; }
note(){ echo "${C}ℹ${Z} $*"; }
warn(){ echo "${Y}⚠${Z} $*"; }

# -------- Args --------
OUT_DIR="licenses"
INCLUDE_BUILD=0
DENY_NONSPDX=0
USE_CARGO_DENY=0
QUIET=0
VERBOSE=0

while (( $# )); do
  case "$1" in
    --out) shift; OUT_DIR="${1:-licenses}";;
    --include-build-deps) INCLUDE_BUILD=1;;
    --deny-nonspdx) DENY_NONSPDX=1;;
    --use-cargo-deny) USE_CARGO_DENY=1;;
    --quiet) QUIET=1;;
    --verbose) VERBOSE=1;;
    -h|--help)
      grep -E '^#( |$)|^#!/' -n "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    *) die "Option inconnue: $1 (voir --help)";;
  esac
  shift
done

# -------- Prechecks --------
command -v cargo >/dev/null 2>&1 || die "cargo introuvable."
if ! command -v cargo-license >/dev/null 2>&1; then
  die "cargo-license introuvable. Installe-le : 'cargo install cargo-license'"
fi

mkdir -p "$OUT_DIR"

DATE_ISO="$(date -u +'%Y-%m-%dT%H:%M:%SZ' || true)"
WS_NAME="$(basename "$(pwd)")"

# Flags cargo-license
CL_AVOID=()
(( INCLUDE_BUILD == 0 )) && CL_AVOID+=(--avoid-build-deps)

# -------- Génération CSV / JSON --------
note "Scan des dépendances (transitives)…"
cargo license "${CL_AVOID[@]}" --csv  > "${OUT_DIR}/THIRD-PARTY.csv"
cargo license "${CL_AVOID[@]}" --json > "${OUT_DIR}/THIRD-PARTY.json"
ok "THIRD-PARTY.csv / .json (transitif) ok"

note "Scan des dépendances directes…"
cargo license "${CL_AVOID[@]}" --csv  --direct-deps-only > "${OUT_DIR}/THIRD-PARTY-direct.csv"
cargo license "${CL_AVOID[@]}" --json --direct-deps-only > "${OUT_DIR}/THIRD-PARTY-direct.json"
ok "THIRD-PARTY-direct.csv / .json (direct) ok"

# -------- Résumé humain --------
NOTE_FILE="${OUT_DIR}/NOTICE-THIRD-PARTY.txt"
TOTAL_ALL=$( (wc -l < "${OUT_DIR}/THIRD-PARTY.csv") 2>/dev/null || echo 0 )
TOTAL_DIR=$( (wc -l < "${OUT_DIR}/THIRD-PARTY-direct.csv") 2>/dev/null || echo 0 )

cat > "$NOTE_FILE" <<EOF
Third-Party Notices — ${WS_NAME}
Generated: ${DATE_ISO}

Files:
  - THIRD-PARTY.csv              (all transitive dependencies)
  - THIRD-PARTY.json
  - THIRD-PARTY-direct.csv       (direct dependencies only)
  - THIRD-PARTY-direct.json

Counts:
  - lines in THIRD-PARTY.csv           : ${TOTAL_ALL}
  - lines in THIRD-PARTY-direct.csv    : ${TOTAL_DIR}

Notes:
  * Edit 'gen_licenses.sh' options if you need to include build-dependencies.
  * For a strict policy, consider:
      cargo install cargo-deny
      cargo deny init   # the first time
      cargo deny check licenses
EOF

ok "Résumé → ${NOTE_FILE}"

# -------- Policy checks (facultatif) --------
FAIL=0
if (( DENY_NONSPDX )); then
  # Cherche ‘unknown’/vide dans les JSON (simple mais efficace)
  if grep -Ei '"license"\s*:\s*("?unknown"?|""|null)' "${OUT_DIR}/THIRD-PARTY.json" >/dev/null; then
    warn "Licences manquantes / unknown détectées (voir THIRD-PARTY.json)."
    FAIL=1
  fi
fi

if (( USE_CARGO_DENY )); then
  if command -v cargo-deny >/dev/null 2>&1; then
    note "Exécution cargo-deny (licences)…"
    # On laisse cargo-deny gérer la politique via deny.toml si présent.
    if ! cargo deny check licenses; then
      warn "cargo-deny a signalé des problèmes de licences."
      FAIL=1
    else
      ok "cargo-deny OK"
    fi
  else
    warn "--use-cargo-deny demandé mais 'cargo-deny' introuvable."
  fi
fi

# -------- Fin --------
if (( FAIL )); then
  echo
  die "Échec politique licences (voir messages ci-dessus)."
fi

echo
ok "Licenses OK — fichiers dans '${OUT_DIR}/'"
exit 0
