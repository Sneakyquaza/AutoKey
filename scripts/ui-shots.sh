#!/bin/sh
# Captures automatiques de l'interface (Linux, Xvfb + police Inter) et comparaison au pixel près.
#   scripts/ui-shots.sh DOSSIER            → captures .ppm + .png dans DOSSIER
#   scripts/ui-shots.sh DOSSIER REFERENCE  → idem, puis compare avec REFERENCE (exit 1 si différence)
# Prérequis : apt install xvfb xauth fonts-inter
set -e
export PATH="$HOME/.cargo/bin:$PATH"
out=$1; ref=$2
[ -n "$out" ] || { echo "usage: $0 DOSSIER [REFERENCE]"; exit 2; }
cd "$(dirname "$0")/.."
cargo build -q --locked --features shots
rm -rf "$out" && mkdir -p "$out/home"
out=$(cd "$out" && pwd)
HOME="$out/home" AUTOKEY_SHOTS="$out" timeout 60 xvfb-run -a -s '-screen 0 1400x1000x24' target/debug/autokey >/dev/null 2>&1 || true
rm -rf "$out/home"
python3 -I scripts/ppm2png.py "$out"/*.ppm
[ -n "$ref" ] || exit 0
fail=0
for f in "$ref"/*.ppm; do
  n=$(basename "$f")
  if cmp -s "$f" "$out/$n"; then echo "OK   $n"; else echo "DIFF $n"; fail=1; fi
done
exit $fail
