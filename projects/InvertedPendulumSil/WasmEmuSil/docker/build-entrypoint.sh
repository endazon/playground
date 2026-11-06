#!/usr/bin/env bash
# docker/Dockerfile.build のエントリポイント。
#   /src  … リポジトリ (読み取り専用バインド。ホストを汚さない)
#   /work … 作業コピー (名前付きボリューム。target/ と node_modules/ が残るので 2 回目以降が速い)
#   /out  … 成果物の持ち出し先 (書き込み可能バインド)
#   /opt/macos-sdk … 任意。macOS SDK を置いたら macos-* もビルドする (Apple の SDK は同梱できない)
#
# コマンド:
#   artifacts [release|debug]   部品 (wasm) / FW / UI バンドル → /out/build/
#   check [--quick]             scripts/check.sh
#   client [--all | --target <os>-<arch> ...]  配布 zip → /out/dist/
#   shell                       bash
#   それ以外                    /work でそのまま実行 (例: cargo run -p ipemu-engine -- ...)
set -euo pipefail

SRC="${IPEMU_SRC:-/src}"
WORK="${IPEMU_WORK:-/work}"
OUT="${IPEMU_OUT:-/out}"
MAC_SDK="${IPEMU_MACOS_SDK_DIR:-/opt/macos-sdk}"

step() { echo; echo "==> $*"; }

sync_src() {
  [ -d "$SRC" ] || { echo "$SRC が見つかりません (scripts/docker.sh 経由で起動してください)" >&2; exit 2; }
  step "ソースを同期: $SRC → $WORK"
  mkdir -p "$WORK"
  # 除外したものは $WORK 側で消されない (rsync は --delete でも除外対象を守る) ので
  # target/ node_modules/ はボリュームに残り、増分ビルドになる
  rsync -a --delete \
    --exclude 'target/' --exclude 'node_modules/' --exclude '**/node_modules/' \
    --exclude 'build/' --exclude 'dist/' \
    --exclude '.git/' \
    --exclude '.vs/' --exclude '.vscode/' --exclude 'store/' --exclude 'dev/store/' \
    "$SRC"/ "$WORK"/
  cd "$WORK"
}

# scripts/check.sh の `gen-ipc-types.sh --check` は git の差分で判定する。
# /work はリポジトリの一部を同期しただけで .git が無いので、使い捨ての索引を作って
# 「同期した内容」を基準にする (生成結果が同期元と違えば差分として出る)。
ensure_git_index() {
  if [ ! -d .git ]; then
    git init -q .
    git config user.email "ipemu@example.invalid"
    git config user.name "ipemu build"
  fi
  git add -A >/dev/null 2>&1 || true
}

copy_out() { # copy_out <work 内の相対パス> [delete|keep]
  local rel="$1" mode="${2:-delete}" del=(--delete)
  [ "$mode" = keep ] && del=()
  [ -d "$WORK/$rel" ] || { echo "持ち出すものがありません: $rel" >&2; return 1; }
  step "成果物を持ち出す: $rel → $OUT/$rel"
  mkdir -p "$OUT/$rel"
  rsync -a ${del[@]+"${del[@]}"} "$WORK/$rel"/ "$OUT/$rel"/
  find "$OUT/$rel" -maxdepth 2 -type f -printf '%10s  %p\n' | sort -k2
}

pnpm_install() { # Electron 本体は要らない場面 (lint / test / UI バンドル)
  step "pnpm install (frozen lockfile)"
  ELECTRON_SKIP_BINARY_DOWNLOAD=1 pnpm install --frozen-lockfile --silent
}

cmd="${1:-artifacts}"; shift || true

case "$cmd" in
  artifacts)
    profile="${1:-release}"
    sync_src
    step "部品 (wasm) をビルド"
    ./scripts/build-components.sh "$profile"
    step "ファームウェア (wasm + native) をビルド"
    ./scripts/build-firmware.sh "$profile"
    pnpm_install
    step "UI バンドル (sim-ui) と管理 / ポータル SPA をビルド"
    pnpm --filter @ipemu/sim-ui build
    pnpm --filter @ipemu/web build
    copy_out build
    ;;

  check)
    sync_src
    ensure_git_index
    step "scripts/check.sh $*"
    ./scripts/check.sh "$@"
    ;;

  client)
    sync_src
    targets=()
    while [ $# -gt 0 ]; do
      case "$1" in
        --all) targets=(linux-x64 linux-arm64 macos-x64 macos-arm64 windows-x64 windows-arm64); shift;;
        --target) targets+=("${2:?--target に値が要ります}"); shift 2;;
        *) echo "client が解釈できない引数: $1" >&2; exit 2;;
      esac
    done
    [ ${#targets[@]} -gt 0 ] || targets=(linux-x64)

    # --cross はまとめて 1 つしか渡せないので、エンジンのビルド方法ごとに package.sh を分けて呼ぶ
    #
    # windows の ABI:
    #   x64   → MSVC ABI (cargo-xwin)。そのまま通る
    #   arm64 → GNU-LLVM ABI (cargo-zigbuild)。MSVC ABI では ring が通らない:
    #     cc-rs は `clang-cl -E <絶対パス>` で MSVC 互換ドライバかを判定するが、MSVC 方式の
    #     ドライバは `/` 始まりの引数をオプションと見るので入力ファイルのパスで必ず失敗する。
    #     判定が転ぶと cc-rs は GNU 方式だと誤認し、cargo-xwin が渡した MSVC 方式の
    #     `/imsvc <dir>` を付けたまま `clang` を呼ぶため全部こける。aarch64 だけ出るのは、
    #     ring がこの構成でだけ GNU 方式のアセンブリ (*-armv8-win64.S) を clang で組み立てるから。
    #     CFLAGS_<triple> での上書きは cargo-xwin が後から潰すので効かない。
    zig=(); xwin=(); wingnu=(); mac=()
    for t in "${targets[@]}"; do
      case "$t" in
        linux-*)        zig+=(--target "$t");;
        windows-arm64)  wingnu+=(--target "$t");;
        windows-*)      xwin+=(--target "$t");;
        macos-*)        mac+=(--target "$t");;
        *) echo "不明な target: $t" >&2; exit 2;;
      esac
    done

    # 1 つの組が落ちても残りは作り、できた分は必ず /out に出す (最後に落ちた組をまとめて報告)
    failed=()
    if [ ${#zig[@]} -gt 0 ]; then
      step "linux 向け (cargo-zigbuild): ${zig[*]}"
      ./scripts/package.sh --cross zigbuild "${zig[@]}" || failed+=("linux")
    fi
    if [ ${#xwin[@]} -gt 0 ]; then
      step "windows 向け (MSVC ABI / cargo-xwin): ${xwin[*]}"
      ./scripts/package.sh --cross xwin "${xwin[@]}" || failed+=("windows-msvc")
    fi
    if [ ${#wingnu[@]} -gt 0 ]; then
      step "windows arm64 向け (GNU-LLVM ABI / cargo-zigbuild): ${wingnu[*]}"
      IPEMU_WIN_TRIPLE_SUFFIX=gnullvm ./scripts/package.sh --cross zigbuild "${wingnu[@]}" \
        || failed+=("windows-gnullvm")
    fi
    # macOS は Apple の SDK が無くてもリンクできる: zig が macOS 用の libc スタブを同梱していて、
    # ipemu-engine は Apple のフレームワークヘッダを使わないため。
    # SDK を渡したときはそれを SDKROOT として使う (将来フレームワークに触る場合や、
    # より本物に近い構成で確かめたい場合)。どちらで作っても未署名・未公証なので、
    # 配布するなら Mac 上で署名し直すこと ([Docs/07](../Docs/07-client-app.md) の「配布 (M6)」)。
    if [ ${#mac[@]} -gt 0 ]; then
      if [ -d "$MAC_SDK" ]; then
        step "macOS 向け (cargo-zigbuild + SDK $MAC_SDK): ${mac[*]}"
        SDKROOT="$MAC_SDK" ./scripts/package.sh --cross zigbuild "${mac[@]}" || failed+=("macos")
      else
        step "macOS 向け (cargo-zigbuild、SDK 無し): ${mac[*]}"
        ./scripts/package.sh --cross zigbuild "${mac[@]}" || failed+=("macos")
      fi
    fi
    copy_out dist keep || true
    if [ ${#failed[@]} -gt 0 ]; then
      echo; echo "!! 失敗した組: ${failed[*]} (できた分は /out/dist に出しています)" >&2
      exit 1
    fi
    ;;

  shell) sync_src; exec bash;;
  *) sync_src; exec "$cmd" "$@";;
esac
echo; echo "done: $cmd"
