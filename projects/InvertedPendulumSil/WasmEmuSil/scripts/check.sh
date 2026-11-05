#!/usr/bin/env bash
# ローカル検証の一括実行 (CI は置かない方針。PR の前にこれを通す)。
#   usage: scripts/check.sh [--quick]   (--quick: clippy と部品の再ビルドを省く)
set -euo pipefail
cd "$(dirname "$0")/.."
QUICK=0; [ "${1:-}" = "--quick" ] && QUICK=1

step() { echo; echo "==> $*"; }

step "cargo fmt --check"
cargo fmt --all -- --check

if [ $QUICK -eq 0 ]; then
  step "cargo clippy (native crates)"
  cargo clippy -p ip-model -p ipemu-core -p ipemu-wasm -p ipemu-engine -p ip-hal -p ip-part-sdk -p ipemu-sign -p ipemu-server --tests -- -D warnings
  step "build components (wasm)"
  ./scripts/build-components.sh release
  step "build firmware (wasm + native)"
  ./scripts/build-firmware.sh release
fi

step "cargo test"
cargo test --workspace --exclude plant-cartpole --exclude motor-driver --exclude encoder --exclude comm-link --exclude fw-balance-rs

step "manifest validate"
cargo run -q -p ipemu-engine -- validate dev/cartpole.manifest.json --no-signature-check

if command -v pnpm >/dev/null 2>&1; then
  step "web (apps/web): typecheck / test / build"
  pnpm install --frozen-lockfile >/dev/null 2>&1 || pnpm install >/dev/null
  (cd apps/web && pnpm test && pnpm build)
else
  echo "(pnpm が無いので apps/web の検証を省略)"
fi

if command -v npx >/dev/null 2>&1 && [ $QUICK -eq 0 ]; then
  step "markdownlint"
  (cd ../../.. && npx --yes markdownlint-cli2 "projects/InvertedPendulumSil/WasmEmuSil/**/*.md") || true
fi
echo; echo "all checks passed"
