#!/usr/bin/env bash
# C 版ファームウェアを wasm (freestanding + ip-cmath) と native (.so/.dylib/.dll) にビルドする。
#   usage: build.sh [OUTDIR]
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="${1:-$ROOT/build/firmware}"
mkdir -p "$OUT" "$HERE/build"
CC="${CC:-clang}"

# wit-bindgen の C バインディング (gen/ はコミット済み。WIT を変えたら再生成する)
if [ ! -f "$HERE/gen/firmware.h" ] || [ ! -f "$HERE/gen/firmware_component_type.o" ]; then
  wit-bindgen c "$ROOT/abi/wit" --world firmware --out-dir "$HERE/gen"
fi

echo "== fw-balance-c (wasm) =="
cargo build --release --target wasm32-unknown-unknown -p ip-cmath --manifest-path "$ROOT/Cargo.toml"
WFLAGS=(--target=wasm32-unknown-unknown -O2 -g -nostdlib -ffreestanding -fvisibility=hidden
        -DIP_HAL_TARGET_WASM -I"$ROOT/abi" -I"$HERE/gen" -I"$HERE/src" -isystem "$HERE/src/shim")
"$CC" "${WFLAGS[@]}" -c "$HERE/src/fw_balance.c" -o "$HERE/build/fw_balance.wasm.o"
"$CC" "${WFLAGS[@]}" -c "$HERE/src/cfg.c" -o "$HERE/build/cfg.wasm.o"
"$CC" "${WFLAGS[@]}" -c "$HERE/src/ip_hal_wasm.c" -o "$HERE/build/ip_hal_wasm.o"
"$CC" "${WFLAGS[@]}" -c "$HERE/gen/firmware.c" -o "$HERE/build/firmware_gen.o"
"$CC" "${WFLAGS[@]}" -fno-builtin -c "$HERE/src/libc_min.c" -o "$HERE/build/libc_min.o"
"$CC" --target=wasm32-unknown-unknown -nostdlib -Wl,--no-entry -Wl,--gc-sections -Wl,--strip-debug \
  "$HERE/build/fw_balance.wasm.o" "$HERE/build/cfg.wasm.o" "$HERE/build/ip_hal_wasm.o" \
  "$HERE/build/firmware_gen.o" "$HERE/build/libc_min.o" "$HERE/gen/firmware_component_type.o" \
  "$ROOT/target/wasm32-unknown-unknown/release/libip_cmath.a" \
  -o "$HERE/build/fw-balance-c.core.wasm"
wasm-tools component new "$HERE/build/fw-balance-c.core.wasm" -o "$OUT/fw-balance-c.wasm"
wasm-tools validate "$OUT/fw-balance-c.wasm"

echo "== fw-balance-c (native) =="
case "$(uname -s)" in
  Darwin) ext=dylib; shared=-dynamiclib;;
  MINGW*|MSYS*|CYGWIN*) ext=dll; shared=-shared;;
  *) ext=so; shared=-shared;;
esac
"$CC" $shared -fPIC -O2 -g -DIP_HAL_TARGET_NATIVE -I"$ROOT/abi" -I"$HERE/src" \
  "$HERE/src/fw_balance.c" "$HERE/src/cfg.c" "$HERE/src/ip_hal_native.c" -lm -o "$OUT/fw-balance-c.$ext"
echo "built: $OUT/fw-balance-c.wasm $OUT/fw-balance-c.$ext"
