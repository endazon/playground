#!/usr/bin/env bash
# IPC 契約 (crates/ipemu-ipc) から TypeScript 型を生成する。
# ts-rs は `cargo test` の中で書き出すので、テストを走らせるだけ。出力は crates/ipemu-ipc/bindings/ (コミットする)。
#   usage: scripts/gen-ipc-types.sh [--check]   (--check: 生成結果が git と一致しなければ非ゼロ終了)
set -euo pipefail
cd "$(dirname "$0")/.."
export TS_RS_LARGE_INT=number   # i64/u64 は number (仮想時刻 ns で 2^53 まで足りる)
cargo test -q -p ipemu-ipc --test export_bindings >/dev/null
if [ "${1:-}" = "--check" ]; then
  if ! git diff --quiet -- crates/ipemu-ipc/bindings; then
    echo "crates/ipemu-ipc/bindings が古いです。scripts/gen-ipc-types.sh を実行してコミットしてください" >&2
    git --no-pager diff --stat -- crates/ipemu-ipc/bindings >&2
    exit 1
  fi
fi
echo "ipc types: crates/ipemu-ipc/bindings/ ($(ls crates/ipemu-ipc/bindings/*.ts | wc -l) files)"
