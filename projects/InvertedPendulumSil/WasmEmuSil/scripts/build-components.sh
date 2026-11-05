#!/usr/bin/env bash
# 部品 (Rust) を wasm コンポーネントにビルドして build/components/ に置く。
#   wasm32-unknown-unknown でコアモジュールを作り、wasm-tools で Component Model の形にする。
#   WASI を一切 import しないので adapter は不要 (決定性の前提)。
#   tcp-reporter (M5 の検証用) は wasi:sockets / wasi:filesystem を import するが、wit-bindgen の
#   component-type セクションに型があるので同じ手順で変換できる (adapter 不要)。
set -euo pipefail
cd "$(dirname "$0")/.."
PROFILE="${1:-release}"
OUT=build/components
# CARGO_TARGET_DIR を共有するとき (worktree での並行作業など) も成果物を見つけられるように
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
mkdir -p "$OUT"
FLAG=""; [ "$PROFILE" = "release" ] && FLAG="--release"
cargo build $FLAG --target wasm32-unknown-unknown \
  -p plant-cartpole -p motor-driver -p encoder -p comm-link -p tcp-reporter
for c in plant-cartpole motor-driver encoder comm-link tcp-reporter; do
  core="$TARGET_DIR/wasm32-unknown-unknown/$PROFILE/${c//-/_}.wasm"
  wasm-tools component new "$core" -o "$OUT/$c.wasm"
  wasm-tools validate "$OUT/$c.wasm"
  printf '%-16s %8d bytes\n' "$c" "$(stat -c %s "$OUT/$c.wasm")"
done
