#!/usr/bin/env bash
# クライアント (Electron シェル + 同梱の ipemu-engine) を未署名 zip にして dist/ に出す (M6, D17 / D64)。
#   usage: scripts/package.sh [--all | --target <os>-<arch> ...] [--skip-engine] [--cross cargo|zigbuild|xwin]
#     --target   linux-x64 | linux-arm64 | macos-x64 | macos-arm64 | windows-x64 | windows-arm64 (繰り返し可)
#     --all      6 通りすべて
#     --cross <cargo|zigbuild|xwin>  ホストと違う target のエンジンのビルド方法 (既定 cargo = その target 用のリンカが要る)。
#                zigbuild: cargo-zigbuild + ziglang (pip install ziglang) で linux / windows-gnu 向け
#                xwin: cargo-xwin で windows-msvc 向け (Microsoft の CRT / SDK を取得する)
#     --skip-engine  engine の cargo ビルドを省き、既存の target/<triple>/release を使う
#   出力: dist/ipemu-client-<version>-<os>-<arch>.zip
#   環境変数: ELECTRON_MIRROR (Electron 本体の取得先。遮断環境は https://npmmirror.com/mirrors/electron/)
#             IPEMU_WIN_TRIPLE_SUFFIX (windows の Rust target の ABI。既定 msvc。gnu / gnullvm にもできる)
#   署名は付けない (署名の足し方は apps/shell/electron-builder.yml の冒頭と Docs/07-client-app.md)
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
TARGETS=(); SKIP_ENGINE=0; CROSS=cargo

while [ $# -gt 0 ]; do
  case "$1" in
    --target) TARGETS+=("${2:?--target に値が要ります}"); shift 2;;
    --all) TARGETS=(linux-x64 linux-arm64 macos-x64 macos-arm64 windows-x64 windows-arm64); shift;;
    --skip-engine) SKIP_ENGINE=1; shift;;
    --cross) CROSS="${2:?--cross に値が要ります}"; shift 2;;
    -h|--help) sed -n 2,15p "$0"; exit 0;;
    *) echo "不明な引数: $1" >&2; exit 2;;
  esac
done
[ ${#TARGETS[@]} -gt 0 ] || { echo "--target か --all を指定してください (-h で使い方)" >&2; exit 2; }

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
WIN_ABI="${IPEMU_WIN_TRIPLE_SUFFIX:-msvc}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
VERSION="$(node -p "require('./apps/shell/package.json').version")"
STAGE="$ROOT/build/pkg"
mkdir -p "$ROOT/dist" "$STAGE"

# <os>-<arch> → Rust triple / electron-builder の引数 / エンジンのファイル名
triple_of() {
  local arch
  case "${1#*-}" in x64) arch=x86_64;; arm64) arch=aarch64;; *) echo "不明な arch: $1" >&2; return 1;; esac
  case "${1%%-*}" in
    linux)   echo "$arch-unknown-linux-gnu";;
    macos)   echo "$arch-apple-darwin";;
    windows) echo "$arch-pc-windows-$WIN_ABI";;
    *) echo "不明な OS: $1" >&2; return 1;;
  esac
}
builder_os_flag() { case "${1%%-*}" in linux) echo --linux;; macos) echo --mac;; windows) echo --win;; esac; }

step() { echo; echo "==> $*"; }

for t in "${TARGETS[@]}"; do triple_of "$t" >/dev/null; done

step "シェルをビルド (main / preload / renderer)"
ELECTRON_SKIP_BINARY_DOWNLOAD=1 pnpm install --frozen-lockfile --silent
pnpm --filter @ipemu/shell run build

for t in "${TARGETS[@]}"; do
  triple="$(triple_of "$t")"
  exe=ipemu-engine; [ "${t%%-*}" = windows ] && exe=ipemu-engine.exe
  out_name="ipemu-client-$VERSION-$t.zip"

  if [ $SKIP_ENGINE -eq 0 ]; then
    step "エンジンをビルド: $triple (release)"
    if [ "$triple" = "$HOST_TRIPLE" ]; then
      cargo build --release -p ipemu-engine
    else
      rustup target add "$triple"
      case "$CROSS" in
        cargo)    cargo build --release -p ipemu-engine --target "$triple";;
        zigbuild) cargo zigbuild --release -p ipemu-engine --target "$triple";;
        xwin)     cargo xwin build --release -p ipemu-engine --target "$triple";;
        *) echo "不明な --cross: $CROSS" >&2; exit 2;;
      esac
    fi
  fi
  if [ "$triple" = "$HOST_TRIPLE" ] && [ -f "$TARGET_DIR/release/$exe" ]; then
    bin="$TARGET_DIR/release/$exe"
  else
    bin="$TARGET_DIR/$triple/release/$exe"
  fi
  [ -f "$bin" ] || { echo "エンジンが見つかりません: $bin" >&2; exit 1; }
  rm -rf "$STAGE/engine/$t" "$STAGE/out/$t"
  mkdir -p "$STAGE/engine/$t"
  cp "$bin" "$STAGE/engine/$t/$exe"

  step "zip を作る: $t → dist/$out_name"
  # arch は electron-builder の --x64 / --arm64。Electron 本体は ELECTRON_MIRROR から取る
  # extraResources.from は apps/shell からの相対で解決されるので相対パスで渡す
  IPEMU_ENGINE_DIR="$(realpath --relative-to="$ROOT/apps/shell" "$STAGE/engine/$t")" \
    pnpm --filter @ipemu/shell exec electron-builder "$(builder_os_flag "$t")" "--${t#*-}" --publish never \
      "-c.directories.output=$STAGE/out/$t" "-c.artifactName=$out_name"
  cp "$STAGE/out/$t/$out_name" "$ROOT/dist/$out_name"
  rm -rf "$STAGE/out/$t"
  echo "$(du -h "$ROOT/dist/$out_name" | cut -f1)  dist/$out_name"
done
echo; echo "done: $(ls "$ROOT/dist" | tr '\n' ' ')"
