#!/usr/bin/env bash
# ファームウェアを wasm (配布形) と native (デバッグ形) の両方にビルドして build/firmware/ に置く。
set -euo pipefail
cd "$(dirname "$0")/.."
PROFILE="${1:-release}"
OUT=build/firmware
mkdir -p "$OUT"
FLAG=""; [ "$PROFILE" = "release" ] && FLAG="--release"

echo "== fw-balance-rs (wasm) =="
cargo build $FLAG --target wasm32-unknown-unknown -p fw-balance-rs
wasm-tools component new "target/wasm32-unknown-unknown/$PROFILE/fw_balance_rs.wasm" -o "$OUT/fw-balance-rs.wasm"
wasm-tools validate "$OUT/fw-balance-rs.wasm"

echo "== fw-balance-rs (native) =="
cargo build $FLAG -p fw-balance-rs
case "$(uname -s)" in
  Darwin) ext=dylib; prefix=lib;;
  MINGW*|MSYS*|CYGWIN*) ext=dll; prefix="";;
  *) ext=so; prefix=lib;;
esac
cp "target/$PROFILE/${prefix}fw_balance_rs.$ext" "$OUT/fw-balance-rs.$ext"

echo "== fw-balance-c =="
firmware/fw-balance-c/build.sh "$OUT"

ls -la "$OUT"
