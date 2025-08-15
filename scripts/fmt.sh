#!/usr/bin/env bash
# scripts/fmt.sh — Formatage automatique du monorepo Vitte
# - Rust      : cargo fmt
# - Vitte     : vitte-fmt (stdin) sur *.vitte / *.vit
# - Node/Docs : Prettier (json, md, yaml, yml, mdx, toml* via plugin si présent)
# - Shell     : shfmt (Bourne shell)
#
# Options:
#   --check        N’écrit rien, vérifie seulement (renvoie code ≠ 0 si diff)
#   --changed      Ne traite que les fichiers modifiés vs HEAD
#   --staged       Ne traite que les fichiers indexés (staged)
#   --rust|--vit|--node|--shell|--docs|--all   Cibles (all par défaut)
#
# Exemples:
#   scripts/fmt.sh --all
#   scripts/fmt.sh --check --changed
#   scripts/fmt.sh --vit --shell
#
# SPDX-License-Identifier: MIT

set -Eeuo pipefail

# ----------------------------- UX & helpers -----------------------------
is_tty() { [[ -t 1 ]]; }
have() { command -v "$1" >/dev/null 2>&1; }

if is_tty && have tput; then
  C_RESET="$(tput sgr0 || true)"
  C_BOLD="$(tput bold || true)"
  C_DIM="$(tput dim || true)"
  C_RED="$(tput setaf 1 || true)"
  C_GREEN="$(tput setaf 2 || true)"
  C_YELLOW="$(tput setaf 3 || true)"
  C_BLUE="$(tput setaf 4 || true)"
else
  C_RESET="" C_BOLD="" C_DIM="" C_RED="" C_GREEN="" C_YELLOW="" C_BLUE=""
fi

say()  { echo -e "${C_BOLD}${C_BLUE}▶${C_RESET} $*"; }
ok()   { echo -e "${C_GREEN}✓${C_RESET} $*"; }
warn() { echo -e "${C_YELLOW}⚠${C_RESET} $*"; }
die()  { echo -e "${C_RED}✗${C_RESET} $*" >&2; exit 1; }

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd -P)"
cd "$ROOT_DIR"

# ----------------------------- Options -----------------------------
CHECK=0
ONLY_CHANGED=0
ONLY_STAGED=0
DO_RUST=0
DO_VIT=0
DO_NODE=0
DO_SHELL=0
DO_DOCS=0

usage() {
  cat <<'EOF'
Usage: scripts/fmt.sh [--check] [--changed|--staged] [--rust|--vit|--node|--shell|--docs|--all]
EOF
}

ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)   CHECK=1; shift;;
    --changed) ONLY_CHANGED=1; shift;;
    --staged)  ONLY_STAGED=1; shift;;
    --rust)    DO_RUST=1; shift;;
    --vit)     DO_VIT=1; shift;;
    --node)    DO_NODE=1; shift;;
    --shell)   DO_SHELL=1; shift;;
    --docs)    DO_DOCS=1; shift;;
    --all)     DO_RUST=1; DO_VIT=1; DO_NODE=1; DO_SHELL=1; DO_DOCS=1; shift;;
    -h|--help) usage; exit 0;;
    *)         ARGS+=("$1"); shift;;
  esac
done
set -- "${ARGS[@]}"

if [[ $DO_RUST$DO_VIT$DO_NODE$DO_SHELL$DO_DOCS == 00000 ]]; then
  DO_RUST=1; DO_VIT=1; DO_NODE=1; DO_SHELL=1; DO_DOCS=1
fi

# Conflit changed/staged
if [[ "$ONLY_CHANGED" == "1" && "$ONLY_STAGED" == "1" ]]; then
  die "--changed et --staged sont exclusifs"
fi

# ----------------------------- Sélection fichiers -----------------------------
git_ls_changed() {
  git diff --name-only --diff-filter=AMCR HEAD --
}
git_ls_staged() {
  git diff --name-only --cached --diff-filter=AMCR --
}

# Utilitaire: produit une liste de fichiers filtrés par extensions, en respectant changed/staged.
# Args: EXT_GLOB (grep -E), FIND_PREDICATE (find -name ... -o ...)
pick_files() {
  local GREP_RE="$1"; shift
  if [[ "$ONLY_STAGED" == "1" ]] && have git && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git_ls_staged | grep -E "$GREP_RE" || true
    return
  fi
  if [[ "$ONLY_CHANGED" == "1" ]] && have git && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git_ls_changed | grep -E "$GREP_RE" || true
    return
  fi
  # fallback: find
  # shellcheck disable=SC2046
  find . -type f \( "$@" \) \
    -not -path "./.git/*" \
    -not -path "./target/*" \
    -not -path "./node_modules/*" \
    -not -path "./dist/*" \
    -print 2>/dev/null || true
}

# ----------------------------- Formatters -----------------------------
format_rust() {
  [[ -f Cargo.toml ]] || { warn "Cargo.toml introuvable — skip Rust"; return; }
  have cargo || die "cargo introuvable"
  if [[ "$CHECK" == "1" ]]; then
    say "Rust — cargo fmt --all -- --check"
    cargo fmt --all -- --check
  else
    say "Rust — cargo fmt --all"
    cargo fmt --all
  fi
  ok "Rust format OK"
}

# vitte-fmt (stdin) fichier par fichier.
vitte_fmt_one() {
  local f="$1"
  local tmp="$(mktemp)"
  if ! have vitte-fmt; then
    die "vitte-fmt introuvable (requis pour --vit)"
  fi
  if [[ "$CHECK" == "1" ]]; then
    if ! cat "$f" | vitte-fmt --stdin >"$tmp" 2>/dev/null; then
      rm -f "$tmp"; return 1
    fi
    if ! cmp -s "$f" "$tmp"; then
      echo "$f"    # rapporter le fichier modifié à l'appelant
      rm -f "$tmp"
      return 2
    fi
    rm -f "$tmp"
    return 0
  else
    if ! cat "$f" | vitte-fmt --stdin >"$tmp"; then
      rm -f "$tmp"; return 1
    fi
    if ! cmp -s "$f" "$tmp"; then
      mv "$tmp" "$f"
      return 0
    fi
    rm -f "$tmp"
    return 0
  fi
}

format_vit() {
  say "Vitte — vitte-fmt (${CHECK:+check})"
  local GREP_RE='(\.vitte|\.vit)$'
  local FILES=()
  while IFS= read -r f; do FILES+=("$f"); done < <(pick_files "$GREP_RE" -name '*.vitte' -o -name '*.vit')
  if [[ "${#FILES[@]}" -eq 0 ]]; then
    warn "Aucun fichier .vitte/.vit"
    return
  fi

  local CHANGED=0
  local FAIL=0
  for f in "${FILES[@]}"; do
    if ! vitte_fmt_one "$f"; then
      warn "vitte-fmt KO: $f"
      FAIL=1
    else
      # vitte_fmt_one renvoie 2 via echo → on l'a capté ? (on a préféré echo pour reporter)
      :
    fi
  done

  if [[ "$CHECK" == "1" ]]; then
    # Refaire un passage pour lister ceux qui diffèrent
    local MODS=()
    for f in "${FILES[@]}"; do
      if ! cat "$f" | vitte-fmt --stdin | cmp -s "$f" -; then
        MODS+=("$f")
      fi
    done
    if [[ "${#MODS[@]}" -gt 0 ]]; then
      printf '%s\n' "${MODS[@]}" | sed 's/^/diff: /'
      die "Vitte format check a trouvé des diffs"
    fi
  else
    [[ "$FAIL" -eq 0 ]] && ok "Vitte format OK" || die "Vitte format a rencontré des erreurs"
  fi
}

# Prettier pour node/docs
format_node_docs() {
  if ! have npx; then
    warn "npx introuvable — skip Prettier"
    return
  fi
  local MODE_ARGS=()
  if [[ "$CHECK" == "1" ]]; then MODE_ARGS=(-c); else MODE_ARGS=(-w); fi

  # Cibles Node (extension VS Code)
  local VS_DIR="editor-plugins/vscode"
  if [[ -d "$VS_DIR" ]]; then
    say "Prettier — extension VS Code (${CHECK:+check})"
    ( cd "$VS_DIR"
      # TS / JSON / JSONC (tmLanguage est JSON), md
      npx --yes prettier "${MODE_ARGS[@]}" \
        "src/**/*.ts" \
        "syntaxes/**/*.json" \
        "language-configuration.json" \
        "snippets/**/*.json" \
        "*.md" || die "Prettier (vscode) KO"
    )
  else
    warn "Extension VS Code absente — skip Node"
  fi

  # Docs/Repo globaux
  say "Prettier — docs & repo (${CHECK:+check})"
  npx --yes prettier "${MODE_ARGS[@]}" \
    "**/*.md" \
    "**/*.json" \
    "**/*.yml" \
    "**/*.yaml" \
    --ignore-path .gitignore \
    --loglevel warn || die "Prettier (repo) KO"

  ok "Prettier OK"
}

# shfmt pour scripts bash
format_shell() {
  have shfmt || { warn "shfmt introuvable — skip shell"; return; }
  say "Shell — shfmt (${CHECK:+check})"
  local FILES=()
  while IFS= read -r f; do FILES+=("$f"); done < <(pick_files '\.sh$' -name '*.sh')

  if [[ "${#FILES[@]}" -eq 0 ]]; then
    warn "Aucun script .sh"
    return
  fi

  if [[ "$CHECK" == "1" ]]; then
    # -d imprime le diff et renvoie code ≠ 0 si diff
    shfmt -d "${FILES[@]}" || die "shfmt diff KO"
  else
    shfmt -w "${FILES[@]}"
  fi
  ok "Shell format OK"
}

# ----------------------------- Orchestration -----------------------------
RC=0
[[ "$DO_RUST"  == "1" ]] && format_rust  || true
[[ "$DO_VIT"   == "1" ]] && format_vit   || true
[[ "$DO_NODE"  == "1" ]] && format_node_docs || true
[[ "$DO_SHELL" == "1" ]] && format_shell || true
[[ "$DO_DOCS"  == "1" ]] && : # déjà pris en charge par Prettier ci-dessus

ok "Formatage terminé ${CHECK:+(check-only)}"
