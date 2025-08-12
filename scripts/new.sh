#!/usr/bin/env bash
set -euo pipefail
NAME="${1:-app}"
TYPE="${2:-cli}"  # cli|web|worker|wasm|embedded
DIR="projects/$NAME"
mkdir -p "$DIR/src" "$DIR/tests"
cat > "$DIR/vitte.toml" <<EOF
[package]
name = "$NAME"
version = "0.1.0"
edition = "2025"
EOF
cat > "$DIR/src/main.vitte" <<'EOF'
fn main(){ print("Hello, Vitte!") }
EOF
echo "[new] created $DIR"
