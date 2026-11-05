#!/usr/bin/env bash
# 部品 (Rust) を wasm コンポーネントにビルドして build/components/ に置く。
#   wasm32-unknown-unknown でコアモジュールを作り、wasm-tools で Component Model の形にする。
#   WASI を一切 import しないので adapter は不要 (決定性の前提)。
set -euo pipefail
cd "$(dirname "$0")/.."
PROFILE="${1:-release}"
OUT=build/components
mkdir -p "$OUT"
FLAG=""; [ "$PROFILE" = "release" ] && FLAG="--release"
cargo build $FLAG --target wasm32-unknown-unknown \
  -p plant-cartpole -p motor-driver -p encoder -p comm-link
for c in plant-cartpole motor-driver encoder comm-link; do
  core="target/wasm32-unknown-unknown/$PROFILE/${c//-/_}.wasm"
  wasm-tools component new "$core" -o "$OUT/$c.wasm"
  wasm-tools validate "$OUT/$c.wasm"
  printf '%-16s %8d bytes\n' "$c" "$(stat -c %s "$OUT/$c.wasm")"
done
