#!/usr/bin/env bash
set -euo pipefail
SRC="${1:-examples/web-echo/main.vitte}"
OUT="${2:-target/bootstrap_app}"
mkdir -p "$(dirname "$OUT")"
# Fake transpile: wrap into C main (placeholder)
cat > "${OUT}.c" <<'C'
#include <stdio.h>
int main(){ puts("Hello from Vitte bootstrap"); return 0; }
C
echo "[ok] wrote ${OUT}.c"
echo "[hint] compile with: cc -O2 ${OUT}.c -o ${OUT}.vitx && strip ${OUT}.vitx"
