#!/usr/bin/env bash
# Procédure de test complète en local (mêmes contrôles que la CI + build Windows + captures UI).
#   scripts/test-all.sh            → tout
#   AUTOKEY_FAST=1 scripts/test-all.sh   → sans les captures UI
# Code retour ≠ 0 au premier échec.
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/.."

WIN=x86_64-pc-windows-gnu
SHOTS_DIR=target/shots-check
t0=$(date +%s)
n=0
fail() { printf '\033[1;31m✘ %s\033[0m\n' "$1"; exit 1; }
step() { n=$((n + 1)); printf '\n\033[1;34m[%s] %s\033[0m\n' "$n" "$1"; }
run() { "$@" || fail "échec : $*"; }

step "cargo fmt --check"
run cargo fmt --check

step "clippy Linux (--features shots, -D warnings)"
run cargo clippy --locked --all-targets --features shots -q -- -D warnings

step "clippy Windows ($WIN)"
run cargo clippy --locked --all-targets --features shots --target "$WIN" -q -- -D warnings

step "cargo test"
run cargo test --locked -q

step "cargo audit"
if cargo audit --version >/dev/null 2>&1; then
  run cargo audit
else
  printf '\033[1;33m⚠ cargo-audit absent : audit des dépendances SAUTÉ (cargo install cargo-audit --locked)\033[0m\n'
fi

step "build release Windows ($WIN)"
run cargo build --release --locked --target "$WIN" -q
exe=target/$WIN/release/autokey.exe
[ -s "$exe" ] || fail "exe introuvable ou vide : $exe"
echo "exe : $exe ($(du -h "$exe" | cut -f1))"

step "captures UI ($SHOTS_DIR)"
if [ "${AUTOKEY_FAST:-0}" = 1 ]; then
  echo "AUTOKEY_FAST=1 : captures sautées"
elif ! command -v xvfb-run >/dev/null 2>&1 || ! command -v python3 >/dev/null 2>&1; then
  printf '\033[1;33m⚠ xvfb-run / python3 absents : captures SAUTÉES (apt install xvfb xauth fonts-inter)\033[0m\n'
else
  expected=$(sed -n 's/^const SCENES: \[&str; \([0-9]*\)\].*/\1/p' src/shots.rs)
  [ -n "$expected" ] || fail "nombre de scènes introuvable dans src/shots.rs"
  run scripts/ui-shots.sh "$SHOTS_DIR"
  got=0; empty=0
  for f in "$SHOTS_DIR"/*.png; do
    [ -e "$f" ] || continue
    got=$((got + 1))
    [ -s "$f" ] || { empty=$((empty + 1)); echo "PNG vide : $f"; }
  done
  [ "$got" -eq "$expected" ] || fail "captures : $got PNG au lieu de $expected"
  [ "$empty" -eq 0 ] || fail "$empty capture(s) vide(s)"
  echo "$got captures non vides"
fi

printf '\n\033[1;32m✔ tout est vert (%ss)\033[0m\n' "$(( $(date +%s) - t0 ))"
